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

## Customer-requested interaction revision (2026-10-30)

Status: implementation authorized by the customer; production start awaits the lead's
explicit slot handoff and updated architecture/UX notes. This checklist supersedes the
old multiple-sketch/New/picker, Sketch snapping and right-pane action-grid workflow.
Earlier green checks describe the previous revision, not acceptance of this interaction.
Source orientation used bounded known files after the lead reported Atlas refresh_required466.
No source changes, builds, tests or browser runs were performed during this preparation.

### Accepted boundaries

One owning Sketch object per page; page-centred initial local origin; no New/picker.
Sketch never calls the Vector snapping engine: only visible, committed axis inference,
explicit endpoint identity reuse and explicitly requested mode relations apply.
Tab enters/cycles numeric input (A); Alt cycles applicable drawing modes once per press;
selection Alt and native text/AltGr behavior remain separate. Keep the existing shell,
left rail and single options bar; constraints are tools, fx owns parameter editing.
The prior source-scaling question is not silently answered by this revision.

### Ordered implementation tasks

1. [ ] **Ownership and first commit (AC1/8–12/19) — document/core tests first.** Implement the approved
   page-owner contract in `curvyo-document-core/src/sketch_commands.rs`, snapshot/codec and
   page/history/copy/clone boundaries identified by the architecture update. Entering or
   cancelling an empty page sketch must not write history; first valid geometry creates
   its owner and graph atomically. Test differently sized/offset pages, page switching,
   first-step undo/redo, delete/recreate, conversion, copy/paste, clones and concurrent
   creation according to the approved deterministic policy. Preserve malformed-record,
   checkpoint provenance and per-record peer-edit guards. Update current-format goldens
   only as specified by the architecture; do not invent migration obligations.
2. [ ] **Session lifecycle and cancellation (AC19/20) — actual event-path tests first.** Refactor
   `session/sketch/{state,lifecycle,commands,creation,keys}.rs` and `session/pages.rs` around
   the active page and transient empty sketch. Centralize unfinished-step cancellation;
   cover press, drag, every multipick phase, preview, numeric draft, tool/page/workbench
   switch, undo and Escape twice. Add explicit secondary-button routing in the thin
   WASM/host boundary: right press/release/contextmenu cancels once, never creates a point;
   idle context menus and Vector actions retain their behavior.
3. [ ] **Axis inference replaces snapping (AC21) — independent core/session assertions.** Remove
   Sketch dependencies on `session/sketch/snapping.rs` and shared snapping state/markers;
   retain Vector snapping unchanged. Implement the documented screen-space axis tolerance,
   preview marks and solver-valid explicit coordinate relations at commit. Test X/Y/both,
   zoom, displaced page origin, constraint inspection/removal and rejection without writes.
   Global snapping enabled/kind/axis settings must have no Sketch effect. Mode-requested
   tangent/perpendicular relations and endpoint connectivity remain distinct from inference.
4. [ ] **Continuous mixed polyline and endpoint resume (AC1/2/20/21).** Extend concrete construction
   APIs and session chain state for the finalized line/arc mode table. Keep stable shared
   endpoints, previous analytic direction and one operation prefix across each preview.
   Test line→tangent arc→line, perpendicular modes, explicit angle, both arc directions,
   closure, selected-endpoint continuation, ambiguous junction refusal and mode changes
   mid-preview. Alt repeat/AltGr must not commit or multiply cycles. Each confirmed segment
   is one history step; cancel keeps previous segments and no unfinished geometry.
5. [ ] **Selection and relation tools (AC2–7/17/22/23).** Extend `curvyo-ui-core/src/sketch.rs` and session
   selection with nearest eligible hits, overlap cycling, preselection and existing
   containment/crossing marquee modifiers. Keep helpers nonselectable and current-ID
   filtering for stale fallback geometry. Relation tools collect/validate references without
   dragging them; preselection remains visible when incompatible. Context dimension derives
   line length, point distance, line angle or explicit circle radius/diameter in Rust;
   annotation placement/value confirmation creates one relation. References remain measured
   read-only; repeated Edit focus and conflict repair stay covered.
6. [ ] **Numeric entry and thin host wiring (AC3/16/18/20/23).** Extend `session/sketch/{numeric,input,view}.rs`
   and `wasm_sketch.rs` with concrete current-step entry data/actions; Rust owns units,
   formulas, dimensions, eligibility and geometry. Tab selects the current relevant value,
   follows visible field order, and Enter commits one valid step then resumes construction.
   Test invalid drafts, native undo, Escape-to-canvas versus next Escape cancellation,
   right-click draft cancellation and key-only mode changes without pointer movement.
7. [ ] **Existing-shell UI revision (AC13–18/19–23).** Update `useSketch`, `useEditorSession`, `Canvas`,
   Sketch components, `PropertiesPanel` and `PanelStrip` using the finalized UX notes.
   Replace native workbench selector with existing styled primitives, preserve compact
   collapsed mode, remove New/picker/internal-ID labels, move creation actions to the left
   grouped rail/top bar, retain lean relationship/status inspection and add the fx tab.
   Keep one options owner, independent annotation/cursor subscriptions and stable focus;
   no extra toolbar, header or diagnostic footer. Test accessible keyboard equivalents.
8. [ ] **Review and final verification (AC1–23).** Run only affected core/document/UI/session tests,
   format, Clippy, current WASM build, frontend type/tests/lint and actual revised browser
   journeys during implementation. Preserve known-answer solver/outline/clone/parity and
   full updated-frame CPU oracles. Hand off to independent tester/UX/architect; run the
   repository gate once at final integration, not after every milestone. Update obsolete
   acceptance expectations through the tester, never weaken them in production work.

### Dependencies to resolve in the incoming architecture/UX notes

- ADR0026 now defines the page-owner root register, original page-ID provenance, lazy
  atomic graph/object/claim, conditional claim cleanup, nonowning clones, occupied-page
  copy refusal and retained diagnosed concurrent losers. Apply that contract to persistence;
  current `create_sketch` has no page owner. Keep format28 and extend current goldens.
- Axis inference needs a supported point-to-local-axis relation: existing Horizontal/Vertical
  relations target a line. Choose the explicit domain representation in architecture,
  not hidden host coordinate edits or arbitrary helper geometry.
- Final mode table defines tangent/perpendicular arc construction and reference-pick stages;
  current chain state stores only first/last point IDs and has no preceding-curve identity.
- Current host forwards secondary presses through the generic pointer API, and Sketch keys
  have no Tab entry action. These are concrete cross-boundary changes, not styling alone.

No new dependency or framework is planned. These are internal milestones of the same
end-to-end story/PR; all accepted geometry, formula, persistence and repair behavior remains.

### Revision ownership milestone (2026-10-30)

- Implemented `Document::{page_sketch, sketch_page, create_page_sketch,
  sketch_ownership_conflict}` and explicit `duplicate_objects_to_page` lower-level copy API.
  Normal Workbench routing is deliberately still pending; raw `create_sketch` remains for
  lower-level unowned fixtures. Customer's dedicated-source-page/linked-clone composition
  steering means the new independent copy API is not a promised composition UI action.
- Original graph provenance and the page claim are separate atomic registers. First geometry
  and claim share one history step at the global page centre. Concurrent losers remain
  readable/diagnosed but refuse graph edits. Delete/conversion clear only their own claim.
- Copies remap entity, constraint, parameter, expression and checkpoint identities; linked
  clones never claim a page. Copy preflight refuses occupied/missing destinations before writes.
- Present malformed claims/provenance fail import; dangling valid references remain repairable.
  Added current-v28 owner, malformed-claim and malformed-assignment goldens. No version bump.
- Object-only conversion undo initially failed (shape restored without claim). It now carries
  only historical page-claim registers naming that original through existing fieldwise restore;
  unrelated root registers and peer-winning claims remain protected.
- Passed `cargo test -p curvyo-document-core --test sketch_pages` (6 tests),
  `cargo test -p curvyo-sketch-core --test remap` (1 test), and current owned-page golden test
  (1 test, three files, rerun without fixture-write mode). Core/document all-test Clippy and
  downstream `cargo check -p curvyo-editor-wasm` passed. Evidence: `target/0066-runtime/revision-*`.
- Pending: independent ownership acceptance; actual Session lazy entry/cancel and paste routing;
  axis relation/inference, mixed modes, selection/numeric entry and the revised host surface.
  The entry policy for a page already containing Vector artwork awaits the customer's answer.

### Revision Session/axis checkpoint (2026-10-30)

- Lazy Session context now holds an empty graph and page-centred placement without a stored
  object. `sketch_construct` creates its original/claim only with completed geometry, in one
  undo step. Page activation cancels pending picks and chooses that page's elected original.
  Clone re-entry activates its original's source page using existing reveal behavior.
- **Proposal A:** entering Sketch on a populated Vector page creates/activates a dedicated
  page without moving or converting content. Existing source-page navigation writes nothing.
  The lower-level `new_sketch` remains temporarily for old callers/tests until the revised
  host removes its action; it now uses page ownership and cannot create a second owner.
- `Relation::OnAxis { point, axis }` uses immutable local X/y=0 and Y/x=0 equations, original
  IDs, strict Point references, ordinary hard residual validation and no reference mode.
  It participates in graph reference traversal, copy remapping and diagnostic labels.
- Sketch creation and drag no longer invoke Vector snapping or extend its target index.
  **Proposal:** axis inference uses a six-screen-pixel band. Only picked newly created points
  receive explicit axis records, never unrelated hidden helper points. Augmented preview
  candidates are independently validated before display; commit remains independently guarded.
- Empty-page preview draws transient subpaths and point glyphs without a fake stored object;
  fixed local axes remain visible and inferred axes receive distinct emphasis.
- `Session::sketch_secondary_press` cancels unfinished state without history. Thin WASM and
  host secondary press/release routing were added; host generated bindings/browser validation
  are pending. Escape after cancellation returns to Sketch Select.
- Passed `cargo test -p curvyo-editor-wasm --lib page_tests` (7), core axis tests (2), full
  affected sketch-core tests (58 before the second axis case was added; both axis cases then
  rerun), affected Rust library Clippy and formatting. Logs: `target/0066-runtime/revision-*`.
- This is not complete revision acceptance: independent Session/axis checks come next;
  continuous mixed modes, Alt cycling, improved selection/context dimension, Tab entry,
  actual host replacement of New/picker and fx panel are still pending. Existing tests that
  explicitly require creation Alt snapping or the old global-origin/New UI need tester-owned
  expectation updates against the revised criteria, not assertion removal.

### Lazy typed edits follow-up

- Independent R02 exposed numeric construction still requiring a stored snapshot. Numeric
  parsing now reads the virtual/current intent graph, then uses the same validated atomic
  materialization as pointer construction. Parameter-first and origin-release edits use this
  path too. Invalid parameter drafts and unchanged fixed-origin edits remain history-free.
- Added `SketchDelta::apply_to` for explicit record edits to caller-owned virtual intent;
  complete graph validation still happens before `create_page_sketch` writes anything.
- Own page/lazy tests: 9/9 passed (`revision-lazy-edits-final.log`). Independent 4/5 currently:
  R02 now passes numeric Point/Circle/radius/undo checks and reaches a formerly unexecuted
  placement oracle incorrectly requiring standalone Point in resolved path anchors. Lead
  confirmed existing Point edit-glyph/output semantics remain; tester owns correction to
  transformed analytic Point assertion. No independent assertion edited by implementation.
- Latest follow-up has not yet rerun Clippy; short independent tester slot follows.

### Mixed-mode / revised host milestone (independent verification next)

- [x] AC13/20: all six continuation modes, shared endpoint closure and reverse endpoint
  continuation; explicit original-ID `PerpendicularAt` relations for curved endpoints.
  Standalone Three-points Arc resumes Line/Arc endpoints. Centre/start/end Arc preserves
  pinned picks, projects final direction and accepts radius only before Start is fixed.
- [x] AC21: axis inference avoids conservatively proven implied line-axis relations; free
  aligned seed coordinates do not imply constraints. Existing user relations remain intact.
- [x] AC22: nearest geometry/overlap cycling, containment/crossing marquee, placed geometry
  query and Alt stroke selection with Shift add/Ctrl remove; selection never writes geometry.
- [x] AC23: dimension label position is finite local metadata keyed by ConstraintId, outside
  the solver fingerprint. Dimension plus position commits once; invalid placement is rejected
  before any write. Copy/delete/convert and current-ID filtering preserve metadata semantics.
- [x] Revised host source: no New/picker; styled workbench selector including collapsed
  alignment, separate fx tab, constraint/dimension tools, mode menus, selected-endpoint Resume,
  on-demand Rust numeric values, Tab/Escape/right-click draft lifecycle and associated errors.
  Numeric drafts reset on selected-entity changes without pointer-rate root presentation updates.
- [x] Own focused evidence: `cargo test -p curvyo-sketch-core` 67/67;
  `cargo test -p curvyo-editor-wasm --lib session::sketch` 42/42;
  UI selection 3/3; document label 1/1; regenerated strict sketch golden cases 4/4.
  Four affected crates `cargo clippy ... --lib --tests -- -D warnings` passed.
  Logs: `target/0066-runtime/revision-{core-final,session-regression,final-clippy}.log`.
- Independent earlier milestones remain document 9/9 and revised Session 8/8. New mode,
  metadata and selection implementation has not yet received independent acceptance.
- New label goldens: `sketch_dimension_label_v28.curvyo` (valid),
  `sketch_label_invalid_v28.curvyo` (invalid); tester owns strict inventory classification.
- [x] Circle dimension preference is ephemeral Rust state: Radius/Diameter changes the
  pending annotation, typed value and committed action consistently without document writes.
- [x] Independent native analytic mode corpus, label metadata and selection tests:
  core 4/4, document 11/11 and Session 11/11; exact evidence is in
  `acceptance-revision-report.md`.
- [ ] Complete independent revised browser journeys and UX review, including numeric
  focus, menu/Alt/Escape, source-page composition and actual rendered controls.
- [ ] Fresh native/WASM parity and actual moving 100-entity complete-frame CPU evidence,
  independent UX/architecture review and final gate. No baseline result substitutes for these.

- Final bounded host checks before tester handoff: `npm run build:wasm`,
  `npx tsc -b --noEmit`, `npm test` (109 test files passed, Node 26.12 summary),
  `npx oxlint` (exit 0, existing warning output), `cargo fmt --all --check`,
  `cargo clippy --target wasm32-unknown-unknown -p curvyo-editor-wasm -- -D warnings`
  all completed successfully. A misplaced bridge documentation comment discovered during
  WASM build was corrected; subsequent strict WASM Clippy passed.
  An isolated intentional `assert.fail` Node test exited 1, confirming assertion failures
  propagate despite the file-level summary (`target/0066-runtime/probes/assertion-failure.log`).
  Current logs: `revision-{wasm-final,tsc-final,oxlint-final,frontend-final-tests,wasm-clippy}.log`.
  No actual browser, final performance or full gate was run in this bounded milestone.

### Revised production-host verification

- Independent revised mode/document/Session corpus passed 4/11/11 cases plus both strict
  fixture sweeps; exact oracles and criteria are in `acceptance-revision-report.md`.
- Reference preference is scoped to dimensional commands and inputs. Subsequent ordinary
  drawing stays editable, and geometric constraints never receive a reference-only flag.
- Actual browser test `frontend/tests/browser/sketch-revision-host.cjs` verifies Radius preview,
  Reference commit, Reference→Fixed, Reference→Point, Tab/Escape, Arc Alt/right cancellation,
  two empty-page draft scopes and parameter-first fx editing. The empty-page assertion first
  failed (`123 mm` survived Add page); page commands now publish the complete editor view,
  not only Document-panel state. The unchanged assertion then passed.
- Screenshot inspection exposed an offscreen source-page origin despite a thin visible frame
  strip. Sketch page changes now reveal an offscreen frame centre; Vector activation rules
  and same-page workbench camera remain unchanged. Focused test first failed at screen
  x=1302.803 for a 1100px canvas; current tests also preserve intentional same-page pan.
- Current affected Rust evidence: Session 44/44, existing pages 7/7, strict native editor
  lib/tests Clippy pass. Fresh WASM/TS/Vite builds and frontend 109-file tests passed before
  the final source-reveal change; final regenerated host artifact follows below.
- Browser execution is initial implementation evidence, not independent final acceptance.
  Performance, current native/WASM parity, remaining independent host matrix and final gate
  remain required. No tolerance, iteration budget or acceptance criterion was relaxed.
- Final production artifact regenerated after the reveal fix: `npm run build:wasm`,
  `npx tsc -b --noEmit`, `npx vite build`, strict editor WASM Clippy and formatting passed.
  Complete host regression passed again against that build. Viewed screenshots:
  `target/0066-browser/revision-dimension.png` (actual radius-reference and Fixed rows),
  `target/0066-browser/revision-host.png` (parameter-first new source page, visible centred
  local origin). Production preview localhost:1421; browser runtime under target/browser-check.
  Logs: `revision-reveal-{wasm,tsc,vite,wasm-clippy}.log`,
  `revision-source-reveal-{red,green}.log` and `target/0066-browser/revision-host.log`.

### Bounded UX consistency correction

- Parameter units now reuse the existing keyboard-operable ToggleGroup (mm/rad/Scalar)
  for drafts and persisted rows; the same Rust commands and validation remain authoritative.
- Workbench selector wrapper uses toolbar chrome with expanded4px vertical/7px horizontal
  inset. Collapsed wrapper remains32px with no inset; narrowed button padding preserves
  the V/S initial beside its chevron. Existing gray menu/focus behavior is unchanged.
- `npx tsc -b --noEmit`, `npm test` (109 files), `npx oxlint` (exit0), and fresh Vite build
  passed. No Rust changes required WASM regeneration. Logs: target/0066-runtime/revision-ux-*.
- Actual browser PASS: keyboard draft unit selection, persisted Scalar2→Length2mm edit,
  absence of native select, expanded/collapsed menu bounds and Escape focus return.
  Radix arrow selection is scheduled: the probe holds key-down through the observable
  selection update before key-up; an instantaneous synthetic press raced that update.
  Generic ToggleGroup behavior was not changed to accommodate the probe.
- Viewed refreshed artifacts under target/0066-browser/ux-revision/fixed-*.png;
  producer-check.cjs and producer-check.log record the exact local browser checks.
  Final production preview remains localhost1421. No parity/performance/full gate run here.

### Actual body-drag performance correction — 2026-10-31

- [x] Diagnose retained strict host RED: actual final-segment body has two coherent point goals;
  single-point native measurements did not exercise that path. Preserve picking/workload.
- [x] Add bounded exact contiguous-suffix certificate with complete hard validation, every raw
  goal checked, deterministic whole-pose branch choice, and unchanged ACS fallback.
- [x] Tests first: body certificate RED then 80 core tests PASS; strict core all-target Clippy PASS.
  Mirrored/reversal analytic witnesses, malformed goals and unsupported topology are covered.
- [x] Architecture source review APPROVE including all-goal finite preflight.
- [x] Fresh browser bindings, TypeScript and production Vite build; unchanged complete-frame
  trace cold11.502ms / all19post-first max7.192ms (`target/0066-browser/revision-suffix`).
- [ ] Independent fresh native/WASM parity, strict host replay and final gate (tester ownership).

### Standalone Arc host first-pick repair — 2026-10-31

- [x] Reproduce unchanged independent `sketch-revision-final-host.cjs` standalone RED.
  The click target was the canvas, not an annotation. A diagnostic-only 700ms pause made
  both endpoint cases pass, identifying stale double-click history across tool changes.
- [x] Reset host press history only when tool/workbench/page/active Sketch/mode values change.
  Pointer-rate view identity and selection do not reset normal same-context double clicks.
- [x] Fresh TypeScript/Vite build; unchanged no-delay standalone phase PASS for Line and Arc
  endpoints, shared IDs, centres and radii (`target/0066-runtime/arc-route-fixed.log`).
- [x] Real dimension annotation double-click plus repeated Edit focus PASS
  (`target/0066-runtime/arc-dimension-smoke.log`); frontend 109 test files PASS; oxlint exit0
  with existing warnings outside the changed hook. No Rust/WASM change in this repair.
- [ ] Independent tester replay and final gate.

### Final revised full gate — 2026-10-31

- [x] Full CLAUDE §7 gate on the local checkout (tester VM unavailable), all stages run with
  `--no-fail-fast`: fmt, workspace Clippy, Nextest 9789/9790 with 130 skipped, ten WASM builds
  and WASM Clippy, cargo deny, rustdoc, frontend WASM/TypeScript/641 tests/oxlint passed.
  The single failure was `acceptance_0050_tester::the_strip_has_layers_fourth_and_never_dimmed`,
  whose tab list predated the accepted fx `parameters` tab (AC7); the list was extended like
  the 0043/0051 lists, nothing else changed. ui-core 2027/2027 then passed.
  Logs: `target/0066-revision-gate/run2/`.
- [x] Rebased onto main 54c74ccf (test/CI-only commits); workspace Clippy, editor-wasm/ui-core/app
  Nextest 6020/6020, TypeScript and frontend 641 tests passed again.
- [ ] Customer acceptance of the story PR.

## Customer revision 2 (2026-10-31)

Source: `specification.md` "Customer revision 2" (AC24-AC35, customer decisions Q1-Q3) and
`ux-notes.md` "Revision 2". Three milestones, each one or two commits on
`story/constraint-sketcher`; the story PR stays open for customer acceptance after M3.
Affected: `curvyo-sketch-core` (tangency), `curvyo-editor-wasm` (`session/sketch`),
`curvyo-ui-core` (eligibility, annotation layout), `curvyo-render-core` (annotation and
symbol drawing), `frontend/src/components/sketch`, `frontend/src/hooks/useEditorSession.ts`.

### M1: chain modes, arc previews, drag of every kind, cursors (AC25, AC26, AC29, AC30)

- [x] 1. AC25: failing session test that Alt cycles all six modes on segment 1 (key repeat
  ignored, start pick kept, preview per mode), then build. **(Proposal)** a free first segment
  takes its direction from the sketch local X axis (the axis AC28 measures from): Tangent line
  persists Horizontal, Perpendicular line persists Vertical, the three arcs start along the
  X / ±Y direction without a persisted relation (no axis entity exists to relate to).
- [x] 2. AC26: failing session test of arc previews on both sides at segments 1, 2, 3, 5, 8
  plus a core test for an arc continuing an arc on its inner side; find and fix the root cause.
- [x] 3. AC26: exactly on the start direction the preview is a straight segment marked
  degenerate (pending annotation, `degenerate` in the view) and commit is refused.
- [x] 4. AC29: session tests that drag point, line, circle, arc, ellipse, rectangle and slot
  members by the edge (whole translation, one history step) and by each defining point, Escape
  mid-drag, before and after a size constraint, and Fixed geometry does not move; arcs inside a
  drawn line-arc-arc chain drag; release-build thread-CPU budget per kind (AC6, 16 ms).
- [x] 5. AC30: Rust decides the Sketch cursor (`cursor` in the Sketch view: `crosshair`,
  `grab`, `grabbing`, `default`); session tests through hover, press, drag, release, Escape;
  host only maps it to CSS in `Canvas.tsx`.
- Root causes. AC25: `set_sketch_mode` refused every mode but Free line without a previous
  curve, so Alt on segment 1 did nothing. AC26: joined arc-arc `Tangent` was always sent to ACS
  as external tangency and `tangency::validate` checked only `d = r1 + r2`; an arc continuing
  an arc on its inner side is internally tangent, so the preview solve failed ("hard
  constraints did not converge") and the frame showed no arc. The side now follows the pose
  (`tangency::internal_joint`, ACS `internal`), which ACS keeps during a solve. No tolerance
  changed. AC29: session drags of every kind already worked; arcs in a drawn arc-arc chain did
  not, for the AC26 reason. Not yet checked in the browser (tester).
- Checks run: red first for `chain_tests` (3 of 3 failing) and the core
  `tangent_arc_after_arc_solves_on_both_sides_of_the_tangent` (red without the core change);
  then `cargo nextest run -p curvyo-editor-wasm -p curvyo-sketch-core` 4020/4020 passed
  (63 skipped), sketch session tests 57/57 after the last split; release
  `sketch_kind_drag_budget` thread CPU max 2.0-2.5 ms per kind; `cargo clippy` both crates
  all targets and wasm32 `-D warnings` clean; `cargo fmt --all --check`; frontend
  `npm run build:wasm`, `npx tsc -b --noEmit`, `npm test` 641 passed, 74 skipped.

### M2: no options bar, Alt for everything, on-canvas label and entry (AC27, AC28, AC33, AC34, AC35)

- [x] 6. AC34: remove the Sketch options bar (`SketchToolbar`: Polyline mode, variant and
  constraint menus, fields, Apply/Preview); drop the `m`/`M` alias (Q1); Alt gated in fields,
  AltGr and Ctrl/Meta/Shift chords; label "Line", "Perpendicular line", "Tangent line",
  "Tangent arc", "Arc left", "Arc right" from Rust, placed per UX notes section 2.
- [x] 7. AC35: Alt cycles Arc (start-first / centre-first), Circle (centre-radius / three
  points) and Ellipse (centre-axes / axis-endpoints) inside one rail tool; after a pick a hint
  instead of a silent discard (AC14); rail has no variant buttons.
- [x] 8. AC33: on-canvas entry box (sibling of `PenEntryChip`, shared `EntryField`), anchored
  at the object per UX notes section 3, Tab/Enter/Escape unchanged (AC20), refusal inside the
  box, Reference row for dimensions.
- [x] 9. AC27: magnetic endpoint capture (12 px in, 16 px out, nearest wins, ties older),
  marker ring and "Endpoint" label, commit reuses the identity, closing yields one point.
- [x] 10. AC28: Ctrl lock to the shared angle stops (`curvyo-ui-core/src/angle_snap.rs`),
  lock cue, capture wins, no effect in tangent/perpendicular modes; 0/180 persist Horizontal,
  90/270 Vertical, pending symbol before commit; test at 0, 90, 180, 270 and 45 degrees.
- Built. Rust (`curvyo-editor-wasm/src/session/sketch/`): `capture.rs` (open endpoints and the
  chain start, hysteresis, ring, "Endpoint" label), `lock.rs` (`snap_angle`, H/V relation,
  pending relation and "31.8 mm, 0°" readout as preview annotations, dashed guide),
  `mode_label.rs` (names, Alt variant partner, quadrant), `entry.rs` (group, fields, live values,
  object anchor and side); the view carries them under `canvas`, which the host keeps out of the
  Properties signature. `curvyo-render-core` gains `build_sketch_capture_marker` and
  `build_sketch_lock_guide` (additive public API; architect review). Host:
  `components/sketch/SketchCanvasChips.tsx` (label, live region, entry box) and
  `placeSketchChip` in `lib/readoutPlacement.ts`; Resume polyline/arc moved to the Sketches
  geometry actions. Removed: `SketchToolbar`, the `m`/`M` alias, the `modes`, `pick_role`,
  `arc_radius` view fields and the `set_sketch_mode`, `set_sketch_circle_dimension`,
  `sketch_numeric_values_json` bindings (no remaining host caller).
- Choices **(Proposal)**: capture 12/16 px as UX notes and the lead's brief give (spec text
  says 10 px); capture targets are open endpoints and the chain start for Polyline and Arc
  (AC27 scope), not centres or the origin; joining an open endpoint ends the chain like
  closing; an unchained first segment or three-point arc ending on a capture is drawn from the
  captured point back to the free start, and a captured end in a non-free first-segment mode is
  drawn free (capture wins); a resumed arc's end pick does not capture (needs an `ArcFrom` with
  an existing end, a core API change). Ctrl lock applies to Polyline free-line and the slot
  centreline (AC28 scope; the slot persists H/V on its `top` side). Alt in the dimension tool
  toggles radius/diameter; the dimension tool shows no mode label.
- Superseded tests: no Rust or node test encoded the bar, dropdown or `M` key; all passed
  unchanged. These tester-owned browser scripts drive the removed options bar ("Sketch tool
  options" form, "Polyline mode", "Sketch tool variant", "Constraint tool", "Circle dimension"
  menus, Apply) and are superseded by AC33-AC35; not rewritten here, left for the tester pass:
  `sketch-revision-host.cjs`, `sketch-revision-final-host.cjs`,
  `sketch-revision-independent.cjs`, `sketch-remaining-acceptance.cjs`, `sketch-field-alt.cjs`,
  `sketch-clone-acceptance.cjs`, `sketch-workbench.cjs`.
- Open for M3: non-dimension constraint tools have no chooser or Apply until the rail card
  (AC24); double-click editing a dimension at its text (AC33 last item) still uses the Sketches
  field until the AC31 annotations exist; the entry box does not yet preview typed values; the
  Lock badge glyph is not drawn (guide, readout and pending relation are).
- Checks run: `cargo nextest run -p curvyo-editor-wasm -p curvyo-render-core -p
  curvyo-sketch-core` 4440/4440 passed (63 skipped), incl. 18 new session tests
  (`capture_tests`, `label_tests`) and 2 render-core tests; the capture and lock tests fail
  with capture or lock disabled (checked by mutation); `cargo clippy` the three crates all
  targets and wasm32 `-D warnings` clean; wasm32 build; `cargo doc -D warnings`;
  `cargo fmt --all --check`; frontend `npm run build:wasm`, `npx tsc -b --noEmit`, `npm test`
  643 passed, 74 skipped, `npx oxlint` no errors. Browser smoke
  `frontend/tests/browser/sketch-rev2-m2.cjs` passed (Chromium, Vite on :1421): no options
  bar, label and six Alt steps, Tab entry box with focus in Length, Escape keeps the pick,
  Ctrl lock with pending Horizontal and "0°" readout, Horizontal persisted, capture closes the
  chain; screenshots in `target/0066-browser/rev2-m2/`.

### M3: rail constraint card, dimensions and symbols on the canvas (AC24, AC31, AC32)

- [x] 11. AC24: "Constraints" rail card in column B (Coincident ... Fixed | Symmetric, Midpoint |
  Dimension), selection-first apply, armed picks otherwise, disabled reasons in tooltips;
  icons redrawn single-colour, licence and attribution recorded (lead).
- [x] 12. AC31: permanent dimension annotations (linear with extension lines and arrows,
  "R 25 mm", "⌀ 50 mm", angle arc), reference shown "(50 mm)" dashed, constant screen size,
  draggable persisted label, hover/selection highlight, double-click edit; never output.
- [x] 13. AC32: constraint symbols next to their geometry, stacked without overlap, accessible
  names, click selects the constraint (AC22), conflict marker (AC5), thinning above 150
  annotations keeps selected/hovered/conflicting (test with 151); fully constrained green with
  a non-colour marker (Proposal).
- [ ] 14. Design-system rows (UX notes "Design-system and implementation notes"), full gate,
  tester pass and UX review before the PR is offered for acceptance.
- M3 build order (implementer): (a) `curvyo-ui-core/src/sketch_tool_use.rs`: per-tool state
  apply / arm / unavailable with reason from the selection (AC24); (b)
  `curvyo-ui-core/src/sketch_dimension_layout.rs`: extension lines, dimension line, arrows or
  ticks, text point and rank for linear, radius, diameter and angle at screen-constant sizes
  (AC31); (c) `curvyo-ui-core/src/sketch_symbol_layout.rs`: badge centres, rows of at most
  three plus "+n", overlap offsets, thinning above 150 with selected/hovered/conflicting kept
  (AC32, test at 151); (d) `curvyo-render-core` draws the dimension marks (driving solid,
  reference dashed 4/3) and the solved-geometry stroke; (e) session: rail states and press,
  armed tools apply on the last eligible pick, annotation hit-test, hover, label drag (one
  history step, Escape restores), double-click opens the entry box at the text; (f) host:
  Constraints card, icon set, DOM dimension text and symbol badges positioned by Rust.
- Built. `curvyo-ui-core`: `sketch_tool_use` (apply / arm / unavailable with reason, valid starts
  found by trying one or two more existing entities), `layout_dimensions` (linear with ranks and
  text slide, radius, diameter, angle; small-span flip and ticks), `layout_symbols` (rows of three
  plus "+n", no overlap with badges or dimension texts, thinning above 150). `curvyo-render-core`:
  `build_sketch_dimension` (driving solid red, reference dashed 4/3 grey, hover band, selected
  accent), `build_sketch_solved`, theme `DIM_DRIVING`, `DIM_REFERENCE`, `SKETCH_SOLVED`. Session
  (`session/sketch/`): `rail.rs` (`sketch_rail_press`, armed tools apply on the last eligible
  pick and stay armed, Escape disarms to Select), `annotations.rs` (dimension records, display
  unit text), `symbols.rs` (glyph per relation, Equal groups numbered in constraint order),
  `annotation_input.rs` (hit-test, hover, text drag with one history step and Escape, double-click
  and Enter open the entry box at the text: `sketch_edit_dimension`, kind `edit`, closes the M2
  open point). Wasm: `sketch_rail_press`, `sketch_edit_dimension`, `sketch_close_dimension_edit`.
  Host: two-card Sketch rail with the Constraints card, `icons/constraintIcons.tsx`,
  `SketchAnnotations` renders Rust-placed dimension texts and symbol badges (pointer input goes to
  the canvas, where Rust hit-tests), entry box kind `edit`, tokens in `index.css`.
- Icons: FreeCAD files fetched at commit `faf7e671` and their metadata read ("FreeCAD LGPL2+");
  single-colour 16px redrawings of their shapes, attribution in `docs/third-party-assets.md`,
  replace-later entry in `docs/technical-debt.md`.
- Choices **(Proposal)**: symbols and dimension texts are DOM elements placed by Rust with
  `pointer-events: none` (accessible names, one icon set for rail and canvas); dimension lines
  and arrows are canvas marks. Geometry within the 6px pick tolerance wins over an annotation
  (keeps drags of small sketches, tester's `suffix_native_body_drag_*`). A click on an annotation
  selects without moving the view. Sweep relations (slot caps) get no canvas dimension; arc
  length uses a radius-style leader without prefix; an arc's radius leader sits at three
  quarters of its sweep (its middle carries the symbols). Coincident, Midpoint and On-axis sit at
  their point; Parallel, Perpendicular, Tangent at the shared point or between the entities;
  Symmetric between its two points. Before the first geometry (no sketch yet) every rail tool is
  unavailable with "Open a sketch first".
- Not done in M3: Sketches rows with 16px glyphs (UX notes table); dotted leaders of pair
  symbols on hover; "+n" opens only through entity selection or preselection. Slot constructions
  reported two of their four tangents as redundant (fixed below, "M3 fixes").
- Checks run: red first only for the session tests of the reveal and geometry-priority fixes (the
  layout and eligibility tests were written with the code). `cargo nextest run -p
  curvyo-editor-wasm -p curvyo-ui-core -p curvyo-render-core -p curvyo-sketch-core` 6500/6500
  passed (84 skipped), incl. 33 new tests (ui-core 18 with the 151-annotation thinning test,
  render-core 3, editor-wasm 12); `cargo clippy` the three changed crates all targets and wasm32
  `-D warnings` clean; wasm32 build; `cargo doc -D warnings`; `cargo fmt --all --check`; frontend
  `npm run build:wasm`, `npx tsc -b --noEmit`, `npm test` 643 passed, 74 skipped, `npx oxlint` no
  errors. Browser smoke `frontend/tests/browser/sketch-rev2-m3.cjs` passed (Chromium, Vite on
  :1421): 11-button Constraints card, rectangle and slot with H/V/equal/tangent symbols,
  selection-first Dimension 50 mm and 25 mm, double-click on the slot radius text opens the entry
  box and sets R 25 mm, a symbol click selects Equal; screenshots in
  `target/0066-browser/rev2-m3/`.

### M3 fixes: dimension text gap, non-redundant slot (AC31, AC32)

- [x] Dimension lines break around their text (UX notes revision 2 §7, "the line does not run
  under the digits"): `layout_dimensions` cuts every line of a dimension (extension, dimension
  line, leader) where it crosses the estimated text box plus a 4px gap (`break_around_text`,
  Liang-Barsky clip). Tests first: `no_dimension_line_runs_under_its_text`,
  `the_dimension_line_breaks_around_a_centred_text_and_still_reaches_both_arrows`; three M3
  layout tests now assert the line reaches the text box edge instead of its centre.
- [x] Slot: Sweep pi on each cap plus a tangent at both ends of each line over-determined the caps
  (top tangent to both caps already makes their diameters parallel, so the bottom pair followed).
  The slot now carries FreeCAD's set: four tangents, one radius, equal caps, centre spacing
  (3 DOF left), no Sweep. **(Proposal)** chosen over dropping two tangents so all four tangent
  symbols stay. Tests first: sketch-core `no_compound_construction_creates_a_redundant_or_
  conflicting_constraint` (rectangle, two slots, closed polyline, circles, ellipses, arc) and
  `a_fillet_creates_...`; session `no_drawing_tool_creates_a_redundant_or_conflicting_constraint_
  on_a_fresh_draw` (every drawing tool, red only for Slot before the fix). Only the slot was
  affected.
- [x] Existing documents: the old golden is kept as `sketch_slot_sweeps_v28.curvyo` and still
  loads and solves to the same points (`a_slot_saved_with_cap_sweeps_still_loads_and_solves`);
  `sketch_slot_v28.curvyo` and `sketch_converted_v28.curvyo` regenerated (no format change).
- Checks (after the review changes below): `cargo nextest run` on ui-core, render-core,
  sketch-core, document-core, editor-wasm 8764/8764 passed (103 skipped); clippy host all
  targets and wasm32 `-D warnings` clean; wasm32 build; `cargo doc -D warnings`; `cargo fmt
  --check`; frontend `build:wasm`, `tsc -b --noEmit`, `npm test` 643 passed / 74 skipped,
  `oxlint` exit 0. Not run: `frontend/tests/browser/sketch-rev2-m3.cjs` (this machine has no
  Playwright browser; the headless shell lacks system `libnspr4`/nss, which need root), so no
  `plate-fixed.png` yet.

### Architect review revision 2 (2026-10-31): required changes 1-7

- [x] 1. `docs/licenses/LGPL-2.1.txt` (verbatim from gnu.org), linked from
  `docs/third-party-assets.md`; that entry and the header of `constraintIcons.tsx` state the file
  is LGPL-2.1-or-later as a whole, not AGPL.
- [x] 2. `build_sketch_solved` and `SOLVED_PX` moved to `curvyo-render-core/src/sketch_overlay.rs`;
  `sketch_annotation_marks.rs` draws dimension marks only.
- [x] 3. One `glyphs::dashed_line(from, to, width, (on, off), color)` for the reference dash
  (4/3) and the lock guide (2/2).
- [x] 4. `Session::set_sketch_circle_dimension` deleted; `radius_preference_updates_pending_
  dimension_without_document_writes` toggles radius/diameter with Alt key presses (there is no M
  key).
- [x] 5. `THINNING_LIMIT` is private to `sketch_symbol_layout.rs`.
- [x] 6. Split: `linear` (arrows/ticks in `span_marks`), `sketch_dimension_records`
  (`sketch_dimension_entry`, `dimension_texts`, `PendingRecord::placed`), `sketch_symbol_records`
  (`sketch_symbol_items`, `PendingSymbol::placed`), `first_segment` (`first_line`). Remaining
  over ~60 lines, justified: `construction_preview.rs` `refresh_sketch_construction_preview`
  (92): one ordered pipeline (picks, command, chain and close overrides, capture join, lock
  relation, preview solve) whose steps share the pending prefix and early returns; splitting it
  would pass six locals through helpers without a second caller. `sketch_preview_fillet` (60) is
  at the limit.
- [x] 7. Text-box helpers (`text_half_px`, `break_around_text`, `box_interval`, `free_text`) in
  `curvyo-ui-core/src/sketch_dimension_text.rs` (110 lines); `sketch_dimension_layout.rs` is at
  428 lines.
- [x] Recommended: named tolerances (`lock.rs` `DIRECTION_TOLERANCE_MM`, `AXIS_STOP_TOLERANCE`;
  `entry.rs` `DIRECTION_TOLERANCE_PX`; `tangency.rs` `DEGENERATE_LENGTH`); `completes` in
  `sketch_tool_use.rs` counts candidates per kind as entities come instead of `for kind in 0..5`;
  dated line in `docs/technical-debt.md` for `session/mod.rs` and `session/keys.rs`. The
  `adrs.md` note stays with the architect.


## Revision 2 acceptance fixes (D2, D3, D4; 2026-10-31)

- [x] D2 (AC32): a thinned dot is hit like its badge and sits where the badge would, clear of
  dimension texts (`layout_symbols` settles dots); hovering the dot, or the geometry of one of
  the constraint's entities, hovers the symbol, and a hovered symbol is kept full.
- [x] D3 (AC32, AC5): symbols carry `redundant` apart from `conflicting`; the accessible name says
  "redundant"; the host draws both with the red border and a "!" corner mark (Proposal).
- [x] D4 (AC33): a Reference field is `readonly`; ticking Reference returns focus to the first
  field, so Enter commits. Root cause of the closing box: the pointer on the box leaves the
  canvas, `pointer_leave` clears the sketch hover and the dimension entry needed it; a placed
  dimension now keeps its entry with the anchor standing in for the pointer, and the host keeps
  the box in place once the pointer is on it.
- Open for the tester: `sketch-remaining-acceptance.cjs` reference phase calls `fill('99')` on
  the field it also requires to be `readonly`; Playwright refuses `fill` on a readonly input
  (8 s timeout). Typing keys (`keyboard.press`) leaves the value and the box unchanged.
- [x] D3 for dimensions: a redundant dimension's annotation carries `redundant` apart from
  `conflicting`, its accessible name ends ", redundant", and it keeps the "! " prefix and red
  box of a conflict (`annotations.rs`).

## Revision 2 UI review changes (ux-notes "Revision 2 UI review", 2026-10-31)

- [x] 1 (AC32, AC31): symbols of a line, circle or arc under 40px on screen show only while
  hovered or selected (`symbols.rs` `SMALL_ENTITY_PX`); radius and diameter texts sit 6px plus
  their half box past the circle, so "R 3.44 mm" never lies on the outline (`leader_text_px`).
  The dimension tiers (56/24/16px) were already in place. Tests first:
  `symbols_of_geometry_under_forty_pixels_show_only_while_hovered_or_selected`,
  `a_radius_text_box_never_lies_on_the_circle`.
- [x] 2 (AC23): the dimension tool measures a point to a segment (either order, preview,
  placement click, entry box "Distance"); one picked point arms the tool; picks that fit no
  kind show "Dimension" plus the reason in red on the mode label. Tests:
  `the_dimension_tool_measures_a_point_to_a_line`,
  `an_ineligible_dimension_pick_says_what_the_tool_needs_on_the_canvas`.
- [x] 3 (AC23, AC34): an arc defaults to Radius, a circle to Diameter; Alt switches; the mode
  label names the kind ("Radius", "Diameter", "Length", "Distance", "Angle"). Test:
  `an_arc_dimensions_its_radius_and_a_circle_its_diameter_and_alt_switches_with_the_label`.
- [x] 4 (AC28): the lock guide is `--accent` at 80% (`SKETCH_LOCK_GUIDE`, **Proposal**; the
  notes' `--axis-guide` read too faint) and is drawn 24px behind the anchor and past the
  pointer only, so the rubber band stays solid; the pending Horizontal/Vertical is the H/V
  badge in preview state at the midpoint (annotation `glyph`, text kept as its name); the Lock
  badge is white with the accent ring and a diagonal arrow (`lock: "angle"`); Sketch no longer
  shows the Vector Ctrl minus badge.
- [x] 5 (AC20, AC33): Rust names the refused field (`canvas.entry.error {field, message}`,
  "Must be more than 0 mm"); only that field gets `aria-invalid` and the red border; the
  message is not repeated in Sketches (`view.error` is null while the box or label shows it);
  the "°" suffix is drawn at the visual size of "mm". Test:
  `a_refused_entry_marks_only_its_field_and_stays_out_of_sketches`.
- [x] 6 (AC7, AC33): a dimension typed into the box that conflicts or leaves the solver without
  a shape stays (inspectable, undoable), its text gets the "!" box ("solver failure" in its
  name, not "conflicting"), and the box stays open on it with the reason. Test:
  `a_dimension_that_leaves_no_valid_shape_keeps_its_box_open_with_the_reason_and_is_marked`.
- [x] 7: a refused canvas pick (e.g. a degenerate tangent arc on closing) is said in red on the
  mode label instead of the Sketches panel. Test:
  `a_refused_pick_is_said_on_the_mode_label_not_only_in_sketches`.
- [ ] 8: **not changed.** AC27 requires the capture marker as "ring plus "Endpoint" label" and
  the tester's `acceptance_0066_rev2.rs` asserts the label; open for the PO/lead.
- [x] 9 (AC32): a badge keeps 2px clear of dimension texts and point glyphs; it first slides
  along its entity (point and spot badges: up) by up to 1.5 pitches, then steps out
  (`layout_symbols_around`; `layout_symbols` keeps its signature). Tests:
  `a_badge_slides_along_its_entity_off_a_dimension_text_before_stepping_out`,
  `a_badge_keeps_clear_of_point_glyphs`, `a_fixed_badge_never_covers_its_point_glyph`.
- [x] 10 (AC5, AC32): a redundant dimension draws its lines dashed (4/3) in its own colour.
  Test: `a_redundant_dimension_is_dashed_in_the_driving_colour`.
- Checks: nextest ui-core, render-core, editor-wasm 6482/6482 passed; clippy `-D warnings`
  host all targets and wasm32; fmt; `cargo doc -D warnings` for the three crates; frontend
  `build:wasm`, `tsc -b --noEmit`, `npm test` 643 passed / 74 skipped, `oxlint` exit 0.
  Browser (production preview): `sketch-rev2-m2.cjs` (now waits for the pending H badge and
  the angle lock badge), `sketch-rev2-m3.cjs`, `sketch-remaining-acceptance.cjs`, and the new
  `sketch-rev2-plate.cjs` (customer's plate: 50 mm, R 25 mm by default radius, 25 mm slot,
  12.5 mm point-to-edge offset) pass; screenshots in `target/0066-browser/rev2-ux-fixed/`.

## Customer revision 3 (2026-10-31)

Source: `specification.md` "Customer revision 3" (AC36-AC41); open questions use their defaults
(R3-Q1 A: point on circle, arc or ellipse; R3-Q2 A: infinite line; R3-Q3 A: 4 px). Visual
conventions: `ux-notes.md` "Revision 2". Affected: `curvyo-sketch-core` (joins, relation
variants), `curvyo-document-core` (golden only, `format_version` stays 28: format 28 is
unreleased and introduced on this branch), `curvyo-ui-core` (eligibility per tool, axis
targets), `curvyo-editor-wasm` (`session/sketch`), `frontend/src/components/sketch`. Every
behaviour is tested through pointer and key events on `Session` (AC41).

- [x] 1. AC36 core: `construct_joined` / `construct_preview_joined` take the existing point each
  pick landed on; the pick's new point takes that identity (also the projected end of a
  centre-first arc). Core tests per construction kind.
- [x] 2. AC36 session: capture on every pick of every point-creating tool and variant (Point,
  Polyline incl. resumed chains and press-drag starts, Arc both variants incl. the resumed and
  centre-first end pick, Circle, Ellipse, Rectangle, Slot); preview and commit reuse the
  identity. Tests first: the customer's arc-refill regression (polyline arc, tangent line,
  tangent arc, tangent arc closed, delete segment 2, Arc centre-first refill) and a
  parametrised end-pick test over every tool and variant.
- [x] 3. AC37 core: `Relation::PointOn` (point on the infinite line of a segment or the full
  circle, arc or ellipse), `Relation::SymmetricAxis` (two points about the X or Y axis),
  `HorizontalPoints` / `VerticalPoints` (two points, AC40), and `Symmetric` whose `axis` may be a
  point (centre). Solver mapping, validation, references, remap; one test per target
  combination asserting the solved position within 1e-6 mm and the DOF change; plate golden
  extended (architect review: new document record variants in format 28).
- [x] 4. AC37/AC40 ui-core and session: Coincident accepts point + point, line, axis, circle,
  arc or ellipse; Symmetric two points + point, line or axis; the X and Y axes are pickable
  targets (pseudo identities, session-only); Horizontal/Vertical two points; Parallel and
  Equal two or more, Fixed one or more, applied as one history step. Kind named in Sketches
  and in the symbol's accessible name.
- [x] 5. AC40: selection-first apply on a rail press in one named step (selection unchanged,
  undo restores); an invalid selection applies nothing, adds no step, arms the tool and shows
  an on-canvas hint naming what it needs. One test per valid combination (result, step count,
  undo).
- [x] 6. AC38: click-to-follow below 4 screen px (press-release-press), press-drag-release at
  4 px or more as today; Escape in the follow state adds no step; capture, Alt and Ctrl in both.
  Test: each tool through both paths gives identical geometry.
- [x] 7. AC39: Tab in the follow state and during a press-drag opens the box with the shape's
  fields (rectangle width/height; slot length, angle, width; circle radius; ellipse both radii;
  line length and angle); only typed fields become driving dimensions. Test: rectangle width
  only gives one width dimension, no height dimension, DOF matching.
- [x] 8. Host wiring (typed-field mask, entry hold during a drag, hint), browser scripts
  `sketch-rev2-m2.cjs`, `sketch-rev2-m3.cjs`, `sketch-rev2-plate.cjs` and a new
  `sketch-rev3.cjs` (arc refill, point + line Coincident); affected-crate gate.
- Built. Core: `curvyo-sketch-core/src/joins.rs` with `construct_joined` /
  `construct_preview_joined` (additive public API); relations `PointOn`, `SymmetricAxis`,
  `HorizontalPoints`, `VerticalPoints`, `Symmetric` with a centre point (ACS: extension `on`,
  an operand circle of equal radius for a point on an arc's full circle, fixed operand axis
  points for the axis mirror, an operand line plus `midpoint` for the centre). ui-core:
  `sketch_relations` (several members, one edit), axis selection identities `@axis-x` /
  `@axis-y` (session only, never stored). Session: capture on every pick (`capture.rs`
  `sketch_joins`), `CLICK_TRAVEL_PX` 4, `typed_dimensions.rs`, `sketch_open_entry`, rail hint.
  Host: `sketch_complete_fields` with the typed mask, `sketch_open_entry` on Tab, unavailable
  rail tools still arm, `point-on` glyph = FreeCAD PointOnObject.
- Document records: four new `Relation` variants and `Symmetric.axis` may name a point;
  `format_version` stays 28 (unreleased, introduced on this branch); `sketch_plate_v28.curvyo`
  extended (and the three goldens derived from it regenerated). Architect review needed.
- Choices **(Proposal)**: a pick creating no point (radius, through, width, ellipse axis) is
  pulled onto the endpoint without a relation; a Point on an endpoint creates nothing; the
  press-drag of every multi-pick tool sets press and release as its first two picks (before:
  release only for three-pick tools); an invalid preselection arms the tool, keeps the
  selection until the first pick and returns `false` (no step); a slot's typed length and width
  replace its built-in spacing and cap dimensions (the slot keeps them when untyped, AC15);
  three points for Symmetric take the last picked as the centre; the rail button keeps
  `aria-disabled` for an unusable selection though a press now arms it.
- Open points: a typed Angle (line, slot) and typed ellipse axes set geometry only: the model
  has no direction-to-axis or ellipse-radius dimension (needs an architect decision);
  selected axes have no highlight yet.
- Superseded own tests: `an_unavailable_tool_does_nothing`, the rail's "coincident
  unavailable for a segment" (now a point-on start), `capture_ignores_..._other_tools`;
  `sketch-rev2-m3.cjs` and `sketch-rev2-plate.cjs` updated for AC37/AC39.
- Checks run: red first for the AC36 tests (joins disabled: 4 of 4 fail), AC38 (Slot drag) and
  the AC40 hint test; `cargo nextest run` sketch-core, ui-core, document-core, editor-wasm
  8480/8480 passed (103 skipped); clippy host all targets and wasm32 `-D warnings`; `cargo doc
  -D warnings`; `cargo fmt --check`; frontend `build:wasm`, `tsc -b --noEmit`, `npm test` 650
  passed / 94 skipped, `oxlint` exit 0. Browser (Vite :1421): `sketch-rev2-m2.cjs`,
  `sketch-rev2-m3.cjs`, `sketch-rev2-plate.cjs`, new `sketch-rev3.cjs` pass; screenshots in
  `target/0066-browser/rev3/`.

### Revision 3 architecture review and acceptance follow-up (2026-10-31)

Tasks (criteria):
- [x] `Relation::SymmetricPoint`; `Symmetric.axis` names a line only (review 1; AC37, AC40).
- [x] `Relation::entity_ids_mut` / `expression_mut` in `references.rs`; `joins` and `remap` use
  them (review 2).
- [x] Negative graph-validation tests for `PointOn`, `Symmetric`, `SymmetricAxis`,
  `HorizontalPoints`/`VerticalPoints`, `SymmetricPoint`, `AxisAngle`, `EllipseRadius`; `@`
  entity IDs reserved (review 3). `validate_records` deliberately does not resolve references
  (merges may dangle), so the refusals are asserted on `Sketch::validate` and on `solve`.
- [x] `EntryField` enum; `sketch_typed_relations` split per construction kind (review 4; AC39).
- [x] `AxisAngle` (ACS `direction`) and `EllipseRadius` (minor: `equal` on `radmin`; major:
  operand point on the major-axis end at the typed distance; ACS 0.1.19 has no major-radius
  relation): typed line/slot Angle and ellipse radii are driving dimensions (review 5; AC39).
  Format stays 28; v28 goldens regenerated; dated note in `adrs.md`.
- [x] Recommended: `CAPTURED`/`PICK_TOLERANCE` as `Length`, reused in `creation.rs`; no
  `Value::Null` relation (returns `SketchError::Invalid`); `On` doc names the difference to
  `PointOn`. Not done: one `(text, typed)` slice for `sketch_complete_fields`, because the
  tester's `acceptance_0066_rev3.rs` calls the two-slice signature.
- [x] AC40 invalid preselection: the hint is set for an unusable selection and for a valid
  start kept as picks; the mode label stands at the selection while the pointer is off canvas.
- [x] AC40 Dimension preselection applies at once (lead decision 2026-10-31, superseding the
  revision 2 arming): a selection a dimension fits (`sketch_tool_use` = Apply) gets a driving
  dimension of the kind `sketch_dimension_action` picks, with the measured value, at the
  default label place, in one history step; the entry box opens on it (`kind: "edit"`) so the
  value can be typed at once; tool and selection stay. No or incomplete selection (one point)
  still arms (pick, then place the label). Own tests replaced:
  `the_dimension_tool_takes_the_kind_the_selection_allows` (now
  `a_dimension_preselection_applies_..._opens_its_entry` and
  `an_incomplete_dimension_selection_arms_...`), `a_placed_dimension_keeps_its_entry_...`
  (arms first, then picks the circle), ui-core `sketch_tool_use` unit test, `sketch-rev2-plate.cjs`.
  Expected-behaviour changes for the tester's `acceptance_0066_rev2.rs` (not edited):
  - `ac24_dimension_rail_tool_arms_with_a_segment_and_creates_a_dimension`: with a segment
    selected the rail press no longer arms; it adds the Length dimension at its measured value
    (one step) and opens the edit box on it. The value is typed with
    `sketch_dimension_text(<new id>, ...)`, not `sketch_apply_action` after a label click (that
    now adds a second, conflicting Length).
  - `rev2_review2_dimension_measures_a_point_to_a_line_in_either_pick_order`: the press with
    point + line selected adds the distance (60 mm measured) at once and returns `true`;
    `dimension_action` is no longer set and no label click follows; typing 40 mm goes through
    the edit box (`sketch_dimension_text`), a second history step, and the point moves to 40 mm.
  - `rev2_review2_an_unsolvable_dimension_is_marked_keeps_its_box_and_can_be_undone`: the
    second press with the dimensioned segment selected adds another Length at the current value
    (redundant with the first, so both are marked); to test an unsolvable value the new box (or
    the first dimension's) is edited to 30 mm with `sketch_dimension_text`.
  - Arming as before needs an empty selection (or one point) and canvas picks.
- [x] Journey 7 (`sketch-revision-independent.cjs`): root cause in the host, a press at the
  spot of a press under 400 ms earlier counted as a double-click although Tab, Escape and a
  rail click came between; leaving the canvas now ends the pairing (`useEditorSession.ts`).
- [x] A drag whose press and release land on one point is a click (follow state, AC38).
- [x] `sketch-rev2-plate.cjs` updated for AC39: the typed 0° already holds the top line (no
  redundant Horizontal) and the spacing label filter skips the new 0° label.

Checks run: `cargo fmt --check`; clippy `-D warnings` host all targets and wasm32 for
sketch-core, ui-core, document-core, editor-wasm; wasm32 build; `cargo doc -D warnings`;
nextest of those crates: after B2 8512/8515, the three failures are the rev2 tests above;
`ac39_known_open_*` passes with `--run-ignored`; frontend `build:wasm`, `tsc -b --noEmit`,
`npm test` 650 passed / 94 skipped, `oxlint` exit 0. Browser (Vite :5173, logs
`target/0066-revision-gate/impl/`): `sketch-rev3`, `sketch-rev2-plate`,
`sketch-revision-independent` (10 journeys) pass; journey 7 fails without the host fix.

### Revision 3 retest fixes (2026-10-31)

Defects from `acceptance-revision-report.md` "Revision 3 retest".

- [x] Defect 1 (AC7, UX notes revision 2 review change 6): a value typed into an edit box
  (at-once box or double-click) that leaves the dimension conflicting or unsolvable keeps the
  box open on it with the reason. Root cause: the edit's document change closes the box
  (`sync_sketch` cancels), and `sketch_dimension_text` read `canvas_edit` only afterwards; it is
  now read first. A conflicting or failed dimension's text shows the value its expression asks
  for (`! 25 mm`), not the kept shape's measure. **(Proposal)** Escape in a box kept open on a
  refused value cancels it: its step is taken back while it is still the latest
  (`refused_step`), so the following undo removes the dimension itself; closing the box any
  other way keeps the dimension inspectable (AC7). Own test:
  `a_typed_value_that_conflicts_keeps_the_box_open_with_the_reason`; tester test
  `rev2_review2_an_unsolvable_dimension_is_marked_keeps_its_box_and_can_be_undone` passes.
- [x] Defect 2 (AC40): a valid but incomplete selection stays as the first picks of every
  constraint tool; only a refused selection is replaced by the first pick (`PickStart`, set by
  the rail press, used by canvas and axis picks). Dimension with one selected point plus a
  picked point (or segment) dimensions at once as a complete preselection would: measured
  value, one step, edit box open, tool back to Select **(Proposal)**. Own tests:
  `a_valid_start_stays_as_the_first_picks_of_every_constraint_tool` (Coincident, Horizontal,
  Vertical, Parallel, Perpendicular, Tangent, Equal, Midpoint, Symmetric incl. axes),
  `one_selected_point_then_dimension_then_a_second_point_dimensions_at_once`; tester test
  `ac40_one_selected_point_then_dimension_then_a_second_point_gives_a_point_distance` passes.

Checks run: `cargo fmt`; clippy `-D warnings` host all targets and wasm32 for editor-wasm;
wasm32 build; nextest editor-wasm 4096/4096; frontend `build:wasm`, `tsc -b --noEmit`,
`npm test` 650 passed / 94 skipped, `oxlint` exit 0. Browser (Vite :5173): `sketch-rev3`,
`sketch-rev2-plate`, `sketch-revision-host` pass.

## Customer revision 4 (2026-10-31)

Source: `specification.md` "Customer revision 4" (AC42-AC48); visual conventions `ux-notes.md`
"Revision 2" (rail card, symbols, dimension anatomy). Affected: `curvyo-sketch-core` (two
relation variants), `curvyo-document-core` (format 29, goldens), `curvyo-ui-core` (actions,
eligibility, projected dimension layout), `curvyo-editor-wasm` (`session/sketch`),
`frontend/src/components/sketch` (glyphs). Every behaviour is tested through pointer and key
events on `Session` (AC41).

- [x] 1. AC44-AC46 core: `Relation::HorizontalDistance` / `VerticalDistance { a, b, value }`,
  signed `b - a` along local X / Y (ACS `horizontal_distance` / `vertical_distance`, a linear
  coordinate difference); the side is kept by storing the left (lower) point first; value 0
  allowed, negative refused (`validation`); `measure` signed. Tests first
  (`curvyo-sketch-core/tests/axis_distance.rs`): 30 mm on a diagonal sets the X projection,
  Y unchanged, DOF minus one; side kept; zero aligns; negative refused; origin; JSON and
  namespace copy.
- [x] 2. Format 29 (lead decision A, 2026-10-31; format 28 shipped in `main` with PR #228):
  `CURRENT_FORMAT_VERSION` 29, new goldens `sketch_*_v29.curvyo` (plate gains a horizontal
  distance over a line's endpoints and a vertical distance between two points); the v28 goldens
  stay as older-version fixtures (`format_28_sketch_goldens_still_open`). Recorded in `adrs.md`.
- [x] 3. AC42: Point, Arc (both), Circle (both), Ellipse, Rectangle, Slot, Fillet and typed
  shapes return to Select with the new shapes selected, one step, draggable (`tool_return.rs`).
  Polyline collects its chain (`chain_created`) and returns to Select with it on close, Enter
  or Escape; Escape with no segment returns to Select with the previous selection.
  Tests: `rev4_return_tests.rs`.
- [x] 4. AC43: a constraint applied by picks or by a rail press, and a placed-and-typed or
  selection-first dimension, return to Select with the referenced geometry selected; a refused
  use leaves the tool armed. Tests: `rev4_apply_tests.rs`, `completes` in
  `rev3_constraint_tests.rs`.
- [x] 5. AC44/AC48 ui-core and session: actions `horizontal-distance` / `vertical-distance`
  (two points, one line, point + origin, point + the axis across the measured direction);
  rail order Dimension, Horizontal distance, Vertical distance; applied at once with the measured
  value and the entry open (selection first, or on the last pick of the armed tool); a selection
  that does not fit arms with the hint. Tests: `rev4_hv_tests.rs`, ui-core `sketch_actions` /
  `sketch_tool_use` tests.
- [x] 6. AC45 annotation: `DimensionGeometry::Projected` (dimension line along the axis beyond
  the outer point, extension lines perpendicular from each point, arrows along the axis, ranks
  and label drag as linear); text "↔ 30 mm" / "↕ 30 mm" (size shown, kind named in the
  accessible name and in Sketches "Horizontal Distance"). Tests: ui-core layout tests,
  `a_typed_horizontal_distance_drives_only_the_x_projection`.
- [x] 7. AC47: Shift-click adds a point, marquee and lasso take the points inside (existing),
  Ctrl-click removes (new, as Ctrl marquee). Test: `two_points_are_selected_by_shift_click_and_by_marquee`.
- [x] 8. Frontend glyphs and `docs/third-party-assets.md`; browser `sketch-rev4.cjs`.

Choices **(Proposal)**:
- FreeCAD-like look for the axis distances: extension lines from both points perpendicular to
  the axis, the dimension line 24 px beyond the outer point, arrows along the axis.
- After a shape, the selection is the created shapes without their defining points (a drawn line
  is selected as the line, as in Vector), so a rail tool applies to it at once; a Point tool point
  is selected itself.
- A Horizontal distance with the X axis (or Vertical with the Y axis) is refused: the distance
  to that axis along the measured direction does not exist.
- The Dimension rail glyph becomes FreeCAD's diagonal `Constraint_Length`, so Dimension,
  Horizontal distance and Vertical distance read apart; the former horizontal ruler is now the
  Horizontal distance glyph.
- A selection-first axis distance keeps the picked geometry selected (a line stays the line,
  not its endpoints).

Tests changed because a revision 4 criterion supersedes them:
- Tester `acceptance_0066_rev2.rs`: `RAIL` gains the two tools (AC44); helpers `draw_line`,
  `line_and_point`, `crowded`, `ac24_each_tool_applies_with_a_valid_preselection` (three points)
  and `ac27_nearest_endpoint_wins_when_several_are_in_range` re-arm the drawing tool or clear the
  selection after each shape, restoring the state they were written against (AC42).
- Tester `acceptance_0066_rev3.rs`: helpers `draw_line`, `place_point` and `scene` likewise (AC42).
- Format oracles asserting 28 as current (format 29): `acceptance_0024_slice2_tester.rs`
  `c109_deterministic_and_format_unchanged`, `drawing_insert.rs` version range,
  `acceptance_0017_block1_tester.rs` damaged-fixture list, `acceptance_0050_tester.rs` named
  sketch goldens, `acceptance_0020_tester_more.rs` explicit-step goldens (v29 names added).
- Own: `an_armed_tool_applies_on_the_last_pick_and_returns_to_select` (was "stays armed",
  AC43), `an_invalid_selection_applies_nothing_then_takes_fresh_picks` (picks stay selected),
  rail count 13, action count 17, `a_dimension_that_leaves_no_valid_shape...` and
  `a_placed_dimension_keeps_its_entry...` re-arm or clear the selection (AC42, AC43).

Checks run: red first for the core axis-distance tests (missing variants), AC42 and AC43
session tests; `cargo fmt --check`; clippy `-D warnings` host all targets and wasm32 for
sketch-core, ui-core, document-core, render-core, editor-wasm; `cargo doc -D warnings`; nextest
sketch-core, ui-core, render-core, document-core 4852/4852, editor-wasm lib 628/628 and all
integration binaries 3504/3504; frontend `build:wasm`, `tsc -b --noEmit`, `npm test` 658 passed /
96 skipped, `oxlint` exit 0. Browser (Vite :1421): `sketch-rev4.cjs`, `sketch-rev3.cjs`,
`sketch-rev2-plate.cjs` pass; screenshots in `target/0066-browser/rev4/`.

### Revision 4 architecture review and acceptance follow-up (2026-11-01)

From `review-session-architecture.md` "Revision 4 review" and `acceptance-revision-report.md`
"Revision 4 acceptance", finding 1.

- [x] R1 (AC46). A value entered on an axis distance orders its points so the solved geometry
  measures non-negative (`input.rs` `keep_axis_side`): a reference distance dragged past its
  other point and made driving at the shown value keeps the dragged side. Test first:
  `a_reference_axis_distance_made_driving_keeps_the_dragged_side`.
- [x] R2 (§5). `dimension_geometry` moves with its helpers to `session/sketch/dimension_geometry.rs`
  and is split per kind (distance, axis distance, diameter, axis angle); 37 lines.
- [x] R3 (§5). The `Projected` layout moves to `curvyo-ui-core/src/sketch_dimension_projected.rs`
  (`#[path]` module); `sketch_dimension_layout.rs` 451 and `annotations.rs` 314 non-test lines.
- [x] Recommended. `a == b` axis distances are refused by validation and by `sketch_relation`
  (origin with the matching axis); damaged v29 golden
  `sketch_axis_distance_negative_v29.curvyo` opens with the sketch stale; the `>= 0` bound is
  commented as a domain bound. Tests first: `a_point_measured_to_itself_is_refused`,
  `axis_distances_store_the_left_or_lower_point_first` (extended),
  `a_negative_axis_distance_golden_opens_stale`.
- [x] Finding 1 (AC42/AC27). Choosing Arc or Polyline resumes only from a selected open endpoint
  (`sketch_can_resume`); a free point left selected by the Point tool stays silent through the
  tool and Alt variant change. Test first: `a_tool_or_variant_change_does_not_resume_from_a_free_point`.

Checks run: red first for R1, a == b (both crates) and finding 1; `cargo fmt --check`; clippy
`-D warnings` host all targets and wasm32 for sketch-core, ui-core, document-core, editor-wasm;
`cargo doc -D warnings`; `cargo deny check`; nextest sketch-core, ui-core, document-core
4449/4449, editor-wasm 4159/4159 (three partitions); frontend `build:wasm`, `tsc -b --noEmit`,
`npm test` 658 passed / 96 skipped, `oxlint` exit 0; browser (Vite :1421) `sketch-rev4.cjs`,
`sketch-rev3.cjs` pass.
