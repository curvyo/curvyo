# Constraint sketcher: delivery plan

Status: Ready for story review — accepted scope and UI implemented; independent acceptance, UX/architecture reviews and full CLAUDE §7 gate passed. Customer acceptance and original-sketch scaling decision remain pending.
Date: 2026-10-30
Basis: `specification.md`, original criteria 1–12 plus customer additions 13–18,
the dimension draft `0070`, and architecture/UX planning review.

Revision: customer feedback of 2026-10-30 supersedes the earlier top-bar-only
Sketch tool proposal. The dedicated view is the central interaction, not an extra
tool in the ordinary vector editor.
Latest layout correction: use the existing Curvyo shell. A Vector/Sketch workbench
selector sits above Properties; Sketches is a tab of that panel. Left floating
tools vary by workbench, and the existing single floating top bar holds active-tool
settings. No additional header, breadcrumb or separate constraint toolbar.

## Intended result

**(Proposal)** Build a complete 2D constraint sketcher inside Curvyo, rather than a
collection of unrelated construction commands. A sketch remains one editable object:
its entities and relationships survive saving, undo and later dimension changes.
Ordinary vector editing remains available outside the sketch.

The accepted delivery includes all eighteen criteria, named parameters and formulas.
Internal milestones do not reduce the end-to-end story scope.

## Demo to agree on

1. Enter the dedicated Sketcher view and draw an 80 × 40 mm plate outline with
   Polyline, then add two circles using different construction methods.
2. Anchor the outline to the sketch origin, constrain its sides horizontally and
   vertically, and dimension its width and height.
3. Make the circles equal, set their diameter to 6 mm, constrain their centres to
   the horizontal centre line, and dimension their separation and position.
4. Show which geometry is still free; add the remaining constraints until the
   sketch reports zero degrees of freedom.
5. Change the plate width to 100 mm and the hole spacing to 60 mm. The constrained
   relationships remain true; only relationships actually specified are preserved.
6. Create a linked clone. Change the hole diameter; both original and clone follow.
7. Add a contradictory dimension. Identify the affected constraints and preserve
   the last valid shape; remove the conflict and resume editing.
8. Save, reopen, edit a dimension, undo and redo it. Convert a copy to ordinary
   paths and export it without constraint symbols or dimension annotations.
9. Add a dimensioned straight slot. Round an outline corner; edit its radius and
   verify that intended dimensions and tangency survive. Undo restores the corner
   and its original references. A conflicting rounding leaves the sketch intact.
10. Leave Sketcher, move the sketch as one object in the document, and re-enter
    it with all relationships intact. Exercise ellipse construction separately.

**Accepted parameter demo:** Name `width` and `edge_margin`; define
`hole_spacing = width - 2 * edge_margin`. Change `width` and show the dependent
dimension and geometry updating. A cyclic or unit-invalid formula keeps the last
valid result and identifies the expression to fix.

A second small example exercises an arc tangent to a line and a circle, with a
radius and an angle dimension. The plate alone does not demonstrate tangency.

## Scope

- Entities: points, lines, polylines, arcs, circles, ellipses, rectangles and
  straight slots. Construction gestures are distinct from canonical geometry:
  centre/radius and three-point circle methods create the same entity kind.
- Construction/editing: dedicated Sketcher tools rather than the ordinary
  Bézier Pen, Polygon or Star; ellipse construction methods; fillets with explicit
  constraint-reference preservation and radius editing.
- Geometric constraints: coincident, horizontal, vertical, parallel,
  perpendicular, tangent, equal, fixed, symmetric and midpoint.
- Numeric dimensions: distance, length, radius, diameter and angle, with units.
- Editing: create/re-enter/leave a sketch, select entities or constraints, apply
  and delete constraints, drag free geometry, edit dimensions directly.
- Diagnosis: degrees of freedom, free/fully constrained/conflicting states,
  redundant constraints and useful conflict identification, with non-colour cues.
- Integration: layers, styling, selection, transforms, history, storage, linked
  clones, snapping to sketch geometry, path conversion and existing output flows.

**Accepted:** Implement the driving/reference dimension foundation required by
the sketch within 0066 and make it reusable by 0070. This does not include the
general Measure tool or every persistent guide workflow of 0070. Stable typed
entity references and measurement semantics live in the shared sketch core.

**Accepted parameter demo:** Named parameters and typed formulas become
a visible part of Sketcher. Parameter evaluation is separate from the geometric
solver: stable references, length/angle/scalar typing, unit conversion, dependency
ordering, cycle diagnostics and last-valid results are required. This expands the
original numeric-only dimension scope and is now accepted by the customer.

Excluded: 3D, assemblies, patterns, automatic
constraint inference, constraint recovery from imported drawings, and a general
CAD kernel. Snapping helps place geometry; it does not silently create constraints.
Curved slots, regular polygon/star tools and implementations of manufacturing,
simulation or extension features are not implied by the requested additions.

## Dedicated view and UX checkpoint — AC1, AC13–18

Design this before frontend wiring or a new view abstraction:

- Vector/Sketch selection above Properties enters/leaves the work context. The
  Sketches property tab identifies the active sketch and its editing scope.
- A view-specific tool rail for selection, point, line/polyline, arc, circle,
  ellipse, rectangle, slot and fillet; construction variants must be discoverable.
- Context controls for the active tool stay in the existing floating top bar.
  Constraints, solver state and parameters live in the Sketches property tab;
  dimensions and constraint symbols remain on the canvas. The former right-panel
  prohibition is superseded; the layout is in `ux-notes.md`.
- Define sketch versus document selection, camera restoration, layer context,
  keyboard focus, cancellation, undo and unfinished work when leaving the view.
- Establish a consistent navigation pattern for later manufacturing workbenches,
  extension-owned views and output/simulation previews. Editing and read-only job
  previews have different permissions and toolsets. No general plugin/view API
  is being implemented before a second concrete consumer exists.

Review a wireframe and the complete user journey with the customer before build.

## Internal milestones

These are implementation checkpoints, not separate infrastructure PRs. The solver
experiment is disposable evidence; production work becomes one end-to-end story
PR against main. Respect the repository's one-checkout and no-stacking rules.

### 1. Prove feasibility before selecting the solver — AC2–7, AC12

- Recheck current solver options and their licences/build requirements using
  primary sources. The old claim that no suitable Rust solver exists is unverified.
- Compare existing candidates with an own pure-Rust implementation before
  selecting either; findings and unknowns are in `solver-research.md`.
- Trial candidates against the exact ten geometric
  constraints and five dimension types above. Specify supported entity combinations;
  “12 constraint kinds” in the old draft is not a sufficient test definition.
- Use free, fully constrained, disconnected, redundant, contradictory and nearly
  singular cases. Include zero-length lines, coincident points, small radii,
  angle wrapping and tangent branches.
- Include general ellipse degrees of freedom, ellipse tangency, near-circular
  orientation degeneracy, slots and fillet radius changes; a lines/circles-only
  success is insufficient for the revised entity scope.
- Demonstrate convergence, useful DOF/rank reporting and conflict identification
  separately. Repeated drags and edits must not flip the solution unexpectedly.
- Compare identical inputs and initial states on native and WASM within 1e-6 mm.
- Measure the stated 100-entity interactive workload using thread CPU time on a
  named platform/build; report difficult cases as well as typical ones. A
  30-entity example does not prove the draft's 16 ms target.
- Record results and rejected options. Do not promise code size, delivery time,
  numerical robustness or the frame budget before this evidence exists.

Checkpoint: review the solver recommendation and evidence before a solver ADR.
If it fails, revise the approach or ask about scope; do not disguise failure by
loosening the acceptance criteria.

### 2. Agree model and interaction — AC1–5, AC7, AC9–11

- Prepare solver/crate and sketch-model/persistence ADRs; document-model choices
  need customer approval. No new crate before its ADR.
- **(Proposal)** Use stable entity/constraint IDs, local sketch coordinates and
  a separate placement transform. Solve each original once; clones reuse the
  solved outline and their existing placement/style rules.
- Define authoritative sketch data, last-valid state/solution branch, derived
  outlines, reference repair, history operations and collaborative edit granularity.
  Do not make two independent representations writable authorities.
- Write UX notes for entry/exit, entity picking, multi-selection, available
  constraints, dimension editing, keyboard operation and conflict repair.
- Implement the dedicated-view UX checkpoint above; do not copy the ordinary
  editor's toolset or assume all constraints fit into the top bar.
- Define stable polyline vertex/segment IDs and slot representation. Compare a
  semantic slot with constrained primitive geometry in the model ADR.
- Define fillet topology edits: preserve unaffected IDs, remap trimmed references
  and retain virtual sharp intersections when needed to preserve dimension intent.
  An original corner coincidence cannot literally connect the new separated
  tangency endpoints; preserve intent or reject the operation without mutation.
- Define infeasible dragging as closest feasible movement while preserving hard
  constraints. Define how the UI distinguishes redundancy, contradiction and
  solver failure; a globally minimal conflict set is not yet promised.

### 3. Build sketch creation and constrained editing — AC1, AC2, AC4–7, AC11–12

- Write core tests first, then implement entities, constraints and diagnostics.
- Wire the dedicated view, Polyline, slot, circle/ellipse construction variants,
  rectangle, origin, geometric selection and drawing.
- Add fillet previews, constraint remapping and radius edits; test rollback,
  removal and undo with dimensions attached to the original corner.
- Add constrained drag previews, cancellation, constraint removal and one history
  step per completed gesture. Invalid attempts preserve the last valid solution.
- Keep modifier handling and snapping consistent across hover, click placement
  and button-down drawing; include key-only changes in browser tests.

### 4. Add dimensions and complete diagnostics — AC3, AC5–7, AC11

- Add all five dimension types, unit parsing and double-click numeric editing.
- Distinguish driving values from reference measurements. Display rounding must
  not alter stored geometry. Reject invalid input without changing the sketch.
- Show accessible status and selectable offending constraints, including when
  a dimension edit introduces a conflict.
- If approved, add the parameter/formula editor with dimensional typing and
  dependency-cycle tests; changes propagate through dimensions into the solver.

### 5. Integrate the document and output — AC8–11

- Store the sketch, constraints and required branch/last-valid state with an
  ADR-defined format bump and golden-file tests. Reopening preserves editability.
- Verify clone chains, placement and style overrides, original deletion and
  conversion, and undo/redo. Clones do not own independent solver graphs.
- Convert to paths in one undo step, preserving closed loops and approximating
  arcs within 0.01 mm. Manufacturing/export flows consume solved geometry only.
- Verify bounds, hit testing, layers, transforms and snapping use the solved
  geometry without introducing a separate geometry interpretation in the UI.

### 6. Independently verify and deliver — AC1–18

- Tester derives acceptance tests from the agreed spec before inspecting code.
- Map every criterion to observable tests; add adversarial solver and native/WASM
  fixtures, persistence goldens and real-browser creation/edit/repair checks.
- Run affected checks during implementation; run the full repository gate once
  before pushing. Architecture and UX reviews precede the ready story PR.
- Demonstrate both examples above; customer acceptance completes the story.

## Decisions to align before production implementation

1. **Extent (accepted):** all AC1–18 including the revised geometry and UI. The
   earlier reduced slice is superseded; patterns remain excluded.
2. **0070:** recommended minimal shared sketch-dimension foundation here, without
   waiting for or also delivering the complete general measurement feature.
3. **Solver (reviewed):** pinned MIT ACS0.1.19 with independent residual guards;
   reproducible feasibility and limitations are in `solver-spike.md`.
4. **Transforms:** proposed translation/rotation preserve local dimensions.
   Scaling the original needs an explicit rule: updating driving values, applying
   an external transform, or converting to paths have different meanings.
   Nonuniformly transformed clones may visually contain ellipses without changing
   the source circle constraint. Do not silently destroy parametric editability.
5. **File readers (accepted AC10):** explicit version refusal, no old-file
   migration or reader retrofit. Explicit conversion produces ordinary paths.
6. **UI (accepted):** existing shell, workbench selector, Sketches Properties tab
   and one floating options bar; see `ui-interaction-map.md`.
7. **Parametric scope (accepted):** named parameters and formulas accompany
   persistent geometric constraints and editable numeric dimensions.
8. **Construction intent:** specify whether three-point creation only constructs
   a circle or also keeps relationships to existing source points. Proposed
   default: temporary creation inputs; persistent references are explicit.
9. **Fillet conflicts:** proposed preservation through reference remapping/virtual
   intersections, otherwise no mutation and an inspectable conflict. Never silently
   delete constraints to make a rounding succeed.

Scope and UI are accepted. Remaining engineering choices are recorded for the
design checkpoint; explicit model ADRs are still required. All solver status and
diagnosis belongs in Sketches Properties, never a bottom state/demo toolbar.

## Current evidence

Specification and related dimension/clone documents reviewed; architecture advice
collected. Scope and UI are accepted. Solver feasibility and the independent 32-test core
checkpoint passed; actual evidence is in `solver-spike.md` and `core-api.md`.
Document checks are recorded below. End-to-end UI acceptance remains open.


## Implementation checklist

- [x] Agree complete geometry, parameter/formula and workbench scope (AC1–18).
- [x] Review static layout against the existing shell; remove bottom status controls (AC18).
- [x] Record independent solver feasibility evidence and select a provisional viable engine (AC2–7, AC12, AC14–17).
- [x] Review solver/core and document-model ADRs before production integration (AC9–12).
- [x] Implement typed sketch model, formulas, constraints and topology-preserving operations with core tests (AC2–7, AC13–17).
- [x] Integrate sketch document storage, outlines, history, clones and conversion (AC8–11).
- [x] Wire workbench selector, Sketches properties, tools, options and canvas interactions (AC1–7, AC13–18).
- [x] Run independent acceptance coverage and UX/architecture reviews (AC1–18).
- [x] Run the full repository gate and prepare one reviewable end-to-end story for its PR (AC1–18).
- [ ] Obtain customer acceptance before rebase-merge and marking the story Done.

### Core implementation sequence (ADR 0025, AC2–7, AC12–17)

- [x] Add pure `curvyo-sketch-core`: typed units, stable IDs, canonical graph records.
- [x] Parse formulas to stable parameter references; evaluate dimensions and cycles.
- [x] Adapt pinned ACS0.1.19 with explicit validation, diagnostic mapping and solve budgets.
- [x] Validate hard residuals and ellipse contact helpers; return only validated candidates.
- [x] Implement analytic outlines, primitive construction, slots and guarded line-corner fillet edits.
- [x] Run affected tests, Clippy and WASM checks; obtain core API review before document integration.

Feasibility evidence: `solver-spike.md`, reproducible `spike/` corpus. Fifty records
pass native/WASM comparisons (four intentional rejection records), 1,596 numeric
values differ by at most 1.43e-14. Moving100 max10.42ms, changed-width P100
with diagnosis max10.48ms; contradictory100 max44.21ms remains an explicit
performance/iteration-budget task. Final AC6/AC12 acceptance is not claimed.

Core checkpoint: `core-api.md` records the actual API, checks and remaining tasks.
Thirty-two core tests (21 implementation + 11 independent acceptance), native
affected Clippy, WASM build/Clippy, format and licence checks passed at the earlier
unpublished core milestone. Architect approved the core boundary before document integration.
Full feature acceptance and production pointer performance remain open.

Document implementation handoff: `document-api.md` records public commands, format28,
checkpoint/merge semantics and exact checks. Core34, document453 unit +6 new
integration tests, clone/history/transform103, UI871/render179 pass. Native and
WASM affected Clippy/build checks pass. Independent document acceptance passed 10 document cases, 32 style inventory cases
and 56 bounded regressions; document all-target Clippy and formatting passed.
The document milestone was reviewed before Session integration. Source scaling remains open. Independent native pointer CPU and actual WASM parity passed later; complete browser-frame AC6 was still pending at that point. Final scoped evidence is below and in host-performance.md.

Session implementation: see `session-api.md` and `session-plan.md`. Core43 tests
and session15 focused tests pass, covering actual pointer creation, feasible drag
release, axis handles, fillet preview revision guards, per-row merged-invalidity
repair, typed entry, and offset-press same-sketch snapping with Alt. Broad editor
483 and UI874 unit tests passed before the final two repair/snapping cases.
The actual frontend workbench is wired. Focused native session checks, frontend TypeScript, 108 frontend tests, lint and browser creation/formula/dimension smoke pass; final browser regressions and independent end-to-end acceptance remain open.

Latest independent evidence: the core matrix now has 19 known-answer cases for
all ten relations, five dimension kinds, reference nonmutation, entity freedom,
fillet radius changes and bounded adversarial inputs. The document matrix has
16 cases, including a primed-cache mutation/undo/import check. Actual native/Node-WASM measurements, full-precision parity and the
separate cold-frame result are in `acceptance-session-report.md`; they are not a
universal frame-time guarantee. UX review approved the existing-shell layout
and corrected read-only reference presentation. Architecture closed the false fully-constrained circle
classification and invalid typed-Fillet stale-preview commit. Extended independent
browser checks now cover construction variants, typed geometry, retained fillet
relations, clone entry/editing, save/open, focus/native undo and Alt/axis masks.
Reference presentation and focused numeric-field Alt inversion passed their
independent browser regressions. Fresh native/Node-WASM chain and conic parity,
all hard-length checks on the saved host drag, and ruler pixel/view invalidation
checks passed. The full CLAUDE §7 gate passed: 9,702 Rust tests; 641 frontend tests passed and 74 skipped; all ten WASM packages built and passed Clippy. Exact remaining checks and warning counts are in acceptance-session-report.md.

### Actual host performance follow-up (AC6)

- [x] Profile and remove repeated document decoding, unchanged modifier solves and root UI reconciliation during annotation-only movement; retain revision/pending-write guards.
- [x] Recheck targeted Rust tests, native Clippy, frontend109 tests, TypeScript, production build and real browser creation/edit/undo workflow.
- [x] Verify the actual complete updated-frame CPU budget independently: 20 inputs, warm maximum15.626ms; every frame after the cold first frame is below16ms. Cold32.884ms is explicitly separate, not claimed under16ms. No tolerance, constraint, precision or budget relaxation.
- [x] Finish the independent browser matrix and fresh native/WASM parity.
- [x] Finish the tester-owned full repository gate before push.

Exact methodology, disjoint worst-frame timings, evidence paths and affected checks are in
[host-performance.md](host-performance.md). The earlier native/Node PASS does not replace this host criterion.

- [x] Reference inspection preserves resolved read-only output; independent reference browser journey passed.
- [x] Cache ruler static layers/dimensions; exact draw-order and60 browser pixel comparisons passed.
- [x] Remove duplicate private input validation/contact decoding; core19+8 tests passed.
- [x] Independent actual-frame confirmation: warm15.626ms, cold32.884ms. Earlier producer15.781/33.762ms and failing traces remain historical evidence in host-performance.md.
