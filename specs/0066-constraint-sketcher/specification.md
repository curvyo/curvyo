# Constraint sketcher: construct geometry from constraints, dimensions and formulas

Status: In progress — customer-authorized UI/interaction revision (2026-10-30); previous gate passed for PR #228, but customer rejected that UI. Revision acceptance and original-sketch scaling decision remain pending.
Priority: Should
Origin: Customer (2026-10-17; expanded and accepted 2026-10-30).

The customer wants editable constrained construction for laser cutting and CNC,
comparable to a SolveSpace sketch or FreeCAD Sketcher. The accepted feature
includes the complete criteria below, including named parameters and formulas;
it is not the original lines/circles-only alternative. The accepted shell uses a
Vector/Sketch workbench selector above Properties, sketch-specific left tools,
one existing floating tool-options bar and a Sketches Properties tab. The
customer's final UI correction removes the demo's bottom state bar: all sketch
states and diagnosis belong in Sketches.

## User value

As a maker cutting and milling parts that must fit, I want to specify dimensions
and geometric relationships once and edit them later, so that changing a plate
width or hole-spacing formula updates the constrained part and its linked clones
without manually repositioning nodes.

## Accepted operations

- **Page sketch:** edit the single Sketch object on its own source page, containing
  construction entities, constraints, dimensions and parameters. No New sketch
  action or multi-sketch picker is required.
- **Compose sketch:** place a Linked clone of the source-page sketch on a composition
  page; edits to the original propagate through the existing clone mechanism.
- **Edit sketch:** enter its Sketch workbench to change its entities and rules.
- **Vector / Sketch:** switch editing context without converting or discarding
  the sketch; switching context alone does not change the document.
- **Apply constraint:** add the selected geometric relationship between valid
  selected sketch entities.
- **Driving dimension:** set a length, distance, radius, diameter or angle that
  the solved geometry must satisfy.
- **Reference dimension:** display a measured value without driving geometry.
- **Parameter / formula:** name a typed value or expression that drives dependent
  parameters and dimensions.
- **Polyline:** create a connected open or closed chain of lines and arcs, switch
  construction mode with Alt, and continue from an explicitly chosen open endpoint.
- **Circle / ellipse variant:** construct the same analytic entity using the
  explicit point-picking method selected in the tool-options bar.
- **Straight slot:** create a centreline-and-width shape with tangent straight
  sides and semicircular caps that remain constrainable.
- **Fillet:** replace a corner with a radius-controlled tangent rounding while
  preserving constraint intent, or refuse the edit without changing the sketch.
- **Inspect:** locate a constraint's entities or inspect the selected geometry's
  freedom, fixing relationships and diagnosis.
- **Remove constraint:** delete the selected relationship and solve the remaining
  sketch; the edit is undoable.
- **Convert to paths:** replace the parametric sketch with ordinary solved paths
  in one undoable edit.

## Acceptance criteria

1. Given a new or existing sketch, then the maker enters **Sketch** using the workbench selector above the right Properties panel and draws points, lines, polylines, arcs, circles, ellipses, rectangles and straight slots. Every resulting entity is constrainable. Selecting **Vector** returns to ordinary document editing with the sketch still editable as one object; re-entering restores its entities and relationships.
2. Given a valid entity selection, then coincident, horizontal, vertical, parallel, perpendicular, tangent, equal, fixed, symmetric and midpoint are discoverable named constraint tools in the existing **left tool rail**, with settings in the existing single top tool-options bar. Sketches Properties inspects existing relationships rather than presenting a wall of creation actions. Tool eligibility and invalid selections are explicit. Applying a valid action adds that relationship and updates the solved result.
3. Given dimensions, then distance, length, radius, diameter and angle support unit-aware values (mm by default), direct double-click editing and an equivalent keyboard-accessible editor in Sketches. Driving dimensions and reference measurements are distinguished. Named parameters and formulas are included in a dedicated **Parameters** Properties tab with an fx symbol: names, expressions, units and resolved values are inspectable, and dependencies propagate into driving dimensions and solved geometry. For example, length parameters `width = 80 mm`, `edge_margin = 10 mm` and `hole_spacing = width - 2 * edge_margin` resolve to 60 mm; changing width to 100 mm resolves to 80 mm. Unknown names, cycles, incompatible units and invalid expressions identify the entry to fix and preserve the last valid values and geometry.
4. Given a newly created page sketch, then its local origin (0, 0) is initially at the page centre (page width / 2, page height / 2), independent of viewport pan/zoom. It is fixed by default and may be explicitly moved or released. Existing geometry is not silently repositioned by page resizing; initial placement and later explicit edits remain distinct.
5. Given the sketch, then the **Sketches** Properties tab shows its inspectable DOF count and text diagnosis. Under-constrained, fully constrained, redundant, conflicting and solver-failure states are distinct. Geometry uses matching visual cues, with colour never the only signal; a valid zero DOF means fully constrained. If no valid count exists, show unavailable rather than a misleading zero. No bottom demo-state bar or separate diagnostic strip is added to the product.
6. Given a drag of free sketch geometry, then the remaining geometry follows its hard constraints at pointer speed (under 16 ms of thread CPU per frame for the 100-entity workload, to be verified by the feasibility spike). A request beyond feasible movement stops at the constraint limit; it does not break constraints. Numerical failure retains the last valid solution and is diagnosed as failure rather than falsely as over-constraint.
7. Given conflicting or redundant constraints, then Sketches identifies the affected constraints, links them to highlighted geometry and allows an explicit edit or removal. Redundancy alone does not imply contradiction. Solver failure does not prove a conflict. A contradictory change retains the last valid shape while the affected relationships remain inspectable; repairs are undoable. No globally minimal conflict-set guarantee is implied without solver evidence.
8. Given Convert to paths, then the result is ordinary paths, with analytic curves approximated as Béziers within 0.01 mm and closed where sketch loops close. Constraint symbols, dimension annotations and parameter controls are not output geometry. Conversion is one history step and undo restores the editable sketch.
9. Given a sketch on its own source page, then composition on another page uses Linked clones. Changing a source dimension or parameter updates those clones under existing placement/style rules. A clone does not become an independently editable constraint system; entering its Sketch editing context opens the source page and original. Composition does not require moving or independently copying the original graph onto the composition page.
10. Given saving and reopening, then geometry, constraints, dimensions, named parameters, formulas and the information needed to restore the last valid editable result survive. The architect decides the authoritative model, format version and reader behaviour in the model ADR before implementation. Curvyo is unreleased SID: no migration or retrofit into existing old readers is required, and no unsupported older-reader fallback is promised.
11. Given a completed geometry creation, constraint edit/removal, dimension edit, parameter/formula edit, fillet or drag, then it creates one named history step. Undo/redo restores geometry and associated relationships and values. Preview, diagnosis and workbench navigation add no document history step.
12. Given identical sketch data and starting configuration, then native and WASM solve results agree within 1e-6 mm; the same condition applies to repeated runs on supported platforms. The feasibility evaluation verifies this independently of upstream claims.
13. Given Polyline, then successive clicks create a continuous connected chain of lines and arcs with shared endpoints, without returning to a new unrelated start after each segment. Alt changes the applicable mode once per key press without losing pending input. Modes cover free line, perpendicular line, tangent line, tangent arc and left/right perpendicular arcs, with angle available as numeric direction. Clicking an eligible existing open line/arc endpoint resumes from that endpoint without a near-coincident duplicate. Clicking the chain start closes it. Enter finishes open when not confirming a numeric step; Escape or right-click discards the live unfinished step and ends continuation, retaining committed segments. Open and closed chains remain constrainable; selected mode intent produces the corresponding valid relationships. The standalone Arc tool also continues from an explicitly picked open line/arc endpoint in its start-first construction, reusing that endpoint identity; a centre-first variant keeps its explicit centre role and offers a discoverable start-first continuation route.
14. Given Circle or Ellipse, then construction variants are explicitly named in the existing floating tool-options bar. Circle supports centre/radius and three-point construction; Ellipse supports centre/axes and axis-endpoints/third-point construction. Previews identify the centre, axes or construction picks. Degenerate input refuses completion without discarding earlier picks needed to correct it. The result remains analytic, constrainable geometry; choosing a construction method alone does not silently create persistent external-reference constraints.
15. Given Slot, then the maker picks its two centreline ends and width to create a straight slot with connected tangent line/arc boundaries. Width and centre spacing are editable, constrainable values. Curved slots are outside this feature.
16. Given a valid corner, then Fillet previews a radius-controlled rounding while preserving applicable constraints and dimensions through explicit entity-reference remapping. If the rounding cannot preserve existing relationships, it identifies affected constraints in Sketches and leaves the sketch unchanged; it never silently drops constraints. Cancellation restores the original corner; commit and undo preserve the associated topology and relationships as one edit.
17. Given constraint inspection, then selecting a constraint locates its entities and selecting geometry allows inspection of remaining freedom or the relationships fixing it. Sketches contains the corresponding status, DOF and diagnostic controls; the canvas highlights the geometry and annotations. Conflict, redundancy and numerical failure remain distinguishable without colour. Exact per-entity classification and conflict diagnostics must be verified in solver evaluation.
18. Given the Sketch workbench, then the existing canvas, rulers, Properties tab strip and floating tool cards remain the application shell. The selector above Properties identifies **Vector / Sketch**; **Sketches** identifies the active page sketch and inspects existing constraints, dimensions and all state/diagnosis; named parameters/formulas live in a separate **Parameters** tab (fx). The left floating tools change with workbench; construction variants and active-tool settings use the existing single floating top bar. There is no additional app header, breadcrumb/Finish bar, second constraint toolbar or bottom state bar. Ordinary Bézier Pen, Star and Polygon are not copied into Sketch by default. Active editing scope and the route back to Vector are explicit. Future work contexts do not authorize implementing manufacturing, simulations or an extension framework here.

19. Given a page, then Sketch workbench directly edits that page's one owning Sketch object, without New sketch or an active-sketch picker. Changing page changes editing scope and cancels only unfinished input. Navigating existing workbench/page contexts creates no history step. Creating a dedicated source page from a populated Vector page is a separate reversible document operation under Proposal A in the revision plan, preserving all existing content. The first durable graph edit can atomically materialize an absent page sketch, including defining a parameter before drawing geometry. Save/reopen, undo/redo, deletion/recreation and concurrent first creation preserve the singleton and stable references. Linked clones remain derived objects under AC9, not additional editable sketch systems. Exact copy/paste/clone reconciliation belongs to ADR0026 and must not silently merge or discard graph intent.
20. Given Sketch construction, then Tab enters its numeric entry with the relevant value selected and Tab/Shift+Tab cycle the visible fields (line length/angle, circle/arc radius/angle as applicable, rectangle width/height). Enter confirms the valid current step; invalid input retains focus and last valid geometry. Escape/right-click never commits geometry. Escape works with canvas, numeric draft, drag and all multi-pick phases; completed operations remain. A subsequent Escape returns to Select. Right-button press/release/contextmenu cannot masquerade as a left click or cancel twice; idle context-menu behavior remains.
21. Given Sketch, then Vector snapping settings, targets, axis masks and held-Alt inversion do not affect Sketch drawing or moving. Automatic inference is limited to local sketch axes: a proposed axis relationship is visible before commit and becomes an inspectable constraint on confirmation (X axis fixes local y = 0; Y axis fixes x = 0; their intersection fixes both). Explicit endpoint connectivity/closure and tangent/perpendicular mode intent are retained, not treated as general snapping. Alt changes modes in drawing context; established Alt selection behavior and native text/system chords remain context-scoped.
22. Given selection, then point/endpoint, analytic edge and dimension/constraint are distinct selectable targets with consistent hover/selection feedback. Shift toggles membership; marquee containment/crossing and Ctrl removal follow existing Curvyo conventions. Tied overlapping targets can be disambiguated without accidental dragging. Constraint tools accept preselection or collect eligible references and do not silently ignore incompatible members. Selecting a relationship highlights its participants; selected geometry can be inspected. Selection is preserved across successful compatible edits and cleared only when its targets cease to exist or the user changes it.
23. Given the dimension tool and selected geometry, then it offers the relevant length, distance, angle or radius/diameter in the existing tool-options bar; alternatively it collects references interactively. A circle/arc selection makes radius editing directly discoverable. The maker positions the annotation and enters a value/expression using the existing numeric-entry style; reference output remains read-only. Existing dimensions edit their expression without adding a duplicate relationship. Constraint creation stays in the existing tool rail and single top bar; no additional floating constraint toolbar is introduced.

## Model and solver readiness

A sketch is an editable document object with stable references between entities,
constraints, dimensions and parameters; its solved outline feeds ordinary vector
selection, styling, transforms and output. The exact authoritative representation,
placement rules, derived geometry and reference repair belong to the model ADR;
this accepted product contract does not substitute for that architectural decision.

The solver/evaluator must run without UI, filesystem, network or clock access in
core and build for native and `wasm32-unknown-unknown`. Parameter evaluation and
geometric solving are distinct responsibilities; a converged numerical result
still requires validation before committing document geometry.

**Evaluation history:** the reproducible ACS0.1.19 spike established a viable
adapter, including auxiliary-contact ellipse tangency, native/WASM agreement and
the measured 100-entity workloads. ADR0025 records its provisional selection;
`solver-spike.md` includes difficult-case latency rather than claiming universal
16 ms performance. Original implementation evidence remains in
`acceptance-core-report.md`, `acceptance-session-report.md` and the architecture
reviews. The customer-authorized interaction revision has separate independent
native and actual-host evidence in `acceptance-revision-report.md`; fresh runtime
parity, performance and the final gate must be completed for that revision.
ADR0026 governs document integration; the source-scaling decision remains pending.
These milestones do not reduce the accepted scope.

## Out of scope

- 3D sketches, assemblies, parametric 3D parts and a general CAD kernel.
- General object/grid/angle snapping in Sketch and unrestricted automatic
  constraint inference. Only the axis inference and explicit construction
  relationships specified in AC13/21 are included.
- Curved slots, patterns, ordinary Star/Polygon tools and imported DXF/SVG
  constraint recovery.
- Spline constraints beyond coincident/fixed control-point relationships.
- The general Measure tool and every guide workflow of `0070`; 0066 includes
  the driving/reference dimension foundation required by the sketch.
- Manufacturing, simulation and extension-workbench implementations.
- Migration of previously saved unreleased files or retrofitting old readers.

## Customer decisions

- **SKETCH-HOME:** built into core, not an extension (2026-10-17).
- **SKETCH-SCOPE:** the complete revised criteria 1–18, including Polyline,
  circles/ellipses, straight slots, constraint-preserving fillets and inspectable
  states; the original reduced first slice is superseded (2026-10-30).
- **SKETCH-PARAMETERS:** named parameters and formulas are included, following
  clarification of “parametric” and the customer's agreement (2026-10-30).
- **SKETCH-UI:** existing shell, Vector/Sketch selector above Properties,
  workbench-specific left tools, one active-tool options bar and Sketches tab;
  all state/diagnosis stays in that tab, with the demo bottom bar removed.
- **SKETCH-START:** implementation is authorized; the pending ACS evidence and
  architect-owned ADRs are prerequisites for committing the production model,
  not a request to reconfirm the accepted feature.

- **SKETCH-SOURCE-PAGE:** a sketch requires its own page; composition on another
  page uses Linked clones (customer, 2026-10-30). The entry action on an already
  populated Vector page is being clarified separately; existing content is preserved.
- **SKETCH-REVISION:** implement the interaction revision after the rejected UI;
  one owning Sketch object per page, origin initially at page centre, no Sketch
  snapping, axis-only automatic inference, Alt mode switching, Tab numeric entry,
  continuous mixed polylines/endpoint continuation, left-rail constraint tools,
  lean inspector and a dedicated fx Parameters tab (customer, 2026-10-30).

The entry flow on a populated Vector page has an explicit safe proposal in the
revision plan; it does not block independent work. Solver selection and model decisions
remain with their documented evaluation/ADR process; there is no default
presumption in favour of an own Newton/LM solver.

## Links

Requirements: R-EDIT-042
Depends on: `0049-linked-clone`, `0016-boolean-operations` (kernel tolerance)
Related: `0070-measure-and-dimension` (shared dimension foundation), `0057-clone-overrides`, `0060-snapping` (Vector only; Sketch uses AC21 axis constraints), `0063-manufacturing-workbenches`
ADRs: `docs/adr/0025-sketch-core-and-solver.md`, `docs/adr/0026-parametric-sketch-nodes-and-workbench.md`; source scaling uses the `needs-customer` process
PR: #228 (revision in progress; not customer-accepted)
