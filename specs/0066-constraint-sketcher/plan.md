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

- [ ] 6. AC34: remove the Sketch options bar (`SketchToolbar`: Polyline mode, variant and
  constraint menus, fields, Apply/Preview); drop the `m`/`M` alias (Q1); Alt gated in fields,
  AltGr and Ctrl/Meta/Shift chords; label "Line", "Perpendicular line", "Tangent line",
  "Tangent arc", "Arc left", "Arc right" from Rust, placed per UX notes section 2.
- [ ] 7. AC35: Alt cycles Arc (start-first / centre-first), Circle (centre-radius / three
  points) and Ellipse (centre-axes / axis-endpoints) inside one rail tool; after a pick a hint
  instead of a silent discard (AC14); rail has no variant buttons.
- [ ] 8. AC33: on-canvas entry box (sibling of `PenEntryChip`, shared `EntryField`), anchored
  at the object per UX notes section 3, Tab/Enter/Escape unchanged (AC20), refusal inside the
  box, Reference row for dimensions.
- [ ] 9. AC27: magnetic endpoint capture (12 px in, 16 px out, nearest wins, ties older),
  marker ring and "Endpoint" label, commit reuses the identity, closing yields one point.
- [ ] 10. AC28: Ctrl lock to the shared angle stops (`curvyo-ui-core/src/angle_snap.rs`),
  lock cue, capture wins, no effect in tangent/perpendicular modes; 0/180 persist Horizontal,
  90/270 Vertical, pending symbol before commit; test at 0, 90, 180, 270 and 45 degrees.

### M3: rail constraint card, dimensions and symbols on the canvas (AC24, AC31, AC32)

- [ ] 11. AC24: "Constraints" rail card in column B (Coincident ... Fixed | Symmetric, Midpoint |
  Dimension), selection-first apply, armed picks otherwise, disabled reasons in tooltips;
  icons redrawn single-colour, licence and attribution recorded (lead).
- [ ] 12. AC31: permanent dimension annotations (linear with extension lines and arrows,
  "R 25 mm", "⌀ 50 mm", angle arc), reference shown "(50 mm)" dashed, constant screen size,
  draggable persisted label, hover/selection highlight, double-click edit; never output.
- [ ] 13. AC32: constraint symbols next to their geometry, stacked without overlap, accessible
  names, click selects the constraint (AC22), conflict marker (AC5), thinning above 150
  annotations keeps selected/hovered/conflicting (test with 151); fully constrained green with
  a non-colour marker (Proposal).
- [ ] 14. Design-system rows (UX notes "Design-system and implementation notes"), full gate,
  tester pass and UX review before the PR is offered for acceptance.
