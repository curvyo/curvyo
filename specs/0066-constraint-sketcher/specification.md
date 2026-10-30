# Constraint sketcher: construct geometry from constraints, dimensions and formulas

Status: Ready for story review (2026-10-30: accepted scope/UI implemented; independent acceptance, UX/architecture reviews and full CLAUDE §7 gate passed; customer acceptance and original-sketch scaling decision pending)
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

- **New sketch:** create an editable set of construction entities, constraints,
  dimensions and parameters in the document.
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
- **Polyline:** create a connected open or closed chain of straight segments.
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
2. Given a valid entity selection, then coincident, horizontal, vertical, parallel, perpendicular, tangent, equal, fixed, symmetric and midpoint are discoverable named constraint actions in the **Sketches** Properties tab, with active-tool shortcuts allowed in the existing tool-options bar. Invalid actions remain focusable and dimmed with a reason. Applying a valid action adds that relationship and updates the solved result.
3. Given dimensions, then distance, length, radius, diameter and angle support unit-aware values (mm by default), direct double-click editing and an equivalent keyboard-accessible editor in Sketches. Driving dimensions and reference measurements are distinguished. Named parameters and formulas are included: names, expressions, units and resolved values are inspectable, and dependencies propagate into driving dimensions and solved geometry. For example, length parameters `width = 80 mm`, `edge_margin = 10 mm` and `hole_spacing = width - 2 * edge_margin` resolve to 60 mm; changing width to 100 mm resolves to 80 mm. Unknown names, cycles, incompatible units and invalid expressions identify the entry to fix and preserve the last valid values and geometry.
4. Given the sketch origin, then it is fixed by default and may be explicitly moved or released.
5. Given the sketch, then the **Sketches** Properties tab shows its inspectable DOF count and text diagnosis. Under-constrained, fully constrained, redundant, conflicting and solver-failure states are distinct. Geometry uses matching visual cues, with colour never the only signal; a valid zero DOF means fully constrained. If no valid count exists, show unavailable rather than a misleading zero. No bottom demo-state bar or separate diagnostic strip is added to the product.
6. Given a drag of free sketch geometry, then the remaining geometry follows its hard constraints at pointer speed (under 16 ms of thread CPU per frame for the 100-entity workload, to be verified by the feasibility spike). A request beyond feasible movement stops at the constraint limit; it does not break constraints. Numerical failure retains the last valid solution and is diagnosed as failure rather than falsely as over-constraint.
7. Given conflicting or redundant constraints, then Sketches identifies the affected constraints, links them to highlighted geometry and allows an explicit edit or removal. Redundancy alone does not imply contradiction. Solver failure does not prove a conflict. A contradictory change retains the last valid shape while the affected relationships remain inspectable; repairs are undoable. No globally minimal conflict-set guarantee is implied without solver evidence.
8. Given Convert to paths, then the result is ordinary paths, with analytic curves approximated as Béziers within 0.01 mm and closed where sketch loops close. Constraint symbols, dimension annotations and parameter controls are not output geometry. Conversion is one history step and undo restores the editable sketch.
9. Given a dimension or parameter changes in a sketch with linked clones, then the clones follow the newly solved shape using the existing clone placement/style rules. A clone does not become an independently editable constraint system.
10. Given saving and reopening, then geometry, constraints, dimensions, named parameters, formulas and the information needed to restore the last valid editable result survive. The architect decides the authoritative model, format version and reader behaviour in the model ADR before implementation. Curvyo is unreleased SID: no migration or retrofit into existing old readers is required, and no unsupported older-reader fallback is promised.
11. Given a completed geometry creation, constraint edit/removal, dimension edit, parameter/formula edit, fillet or drag, then it creates one named history step. Undo/redo restores geometry and associated relationships and values. Preview, diagnosis and workbench navigation add no document history step.
12. Given identical sketch data and starting configuration, then native and WASM solve results agree within 1e-6 mm; the same condition applies to repeated runs on supported platforms. The feasibility evaluation verifies this independently of upstream claims.
13. Given Polyline, then successive clicks create connected straight segments with shared coincident endpoints. Enter finishes an open chain; clicking the start completes a closed chain; Escape cancels the unfinished segment without discarding already completed segments. Open and closed chains remain constrainable.
14. Given Circle or Ellipse, then construction variants are explicitly named in the existing floating tool-options bar. Circle supports centre/radius and three-point construction; Ellipse supports centre/axes and axis-endpoints/third-point construction. Previews identify the centre, axes or construction picks. Degenerate input refuses completion without discarding earlier picks needed to correct it. The result remains analytic, constrainable geometry; choosing a construction method alone does not silently create persistent external-reference constraints.
15. Given Slot, then the maker picks its two centreline ends and width to create a straight slot with connected tangent line/arc boundaries. Width and centre spacing are editable, constrainable values. Curved slots are outside this feature.
16. Given a valid corner, then Fillet previews a radius-controlled rounding while preserving applicable constraints and dimensions through explicit entity-reference remapping. If the rounding cannot preserve existing relationships, it identifies affected constraints in Sketches and leaves the sketch unchanged; it never silently drops constraints. Cancellation restores the original corner; commit and undo preserve the associated topology and relationships as one edit.
17. Given constraint inspection, then selecting a constraint locates its entities and selecting geometry allows inspection of remaining freedom or the relationships fixing it. Sketches contains the corresponding status, DOF and diagnostic controls; the canvas highlights the geometry and annotations. Conflict, redundancy and numerical failure remain distinguishable without colour. Exact per-entity classification and conflict diagnostics must be verified in solver evaluation.
18. Given the Sketch workbench, then the existing canvas, rulers, Properties tab strip and floating tool cards remain the application shell. The selector above Properties identifies **Vector / Sketch**; **Sketches** identifies the active sketch and holds constraints, dimensions, parameters and all state/diagnosis. The left floating tools change with workbench; construction variants and active-tool settings use the existing single floating top bar. There is no additional app header, breadcrumb/Finish bar, second constraint toolbar or bottom state bar. Ordinary Bézier Pen, Star and Polygon are not copied into Sketch by default. Active editing scope and the route back to Vector are explicit. Future work contexts do not authorize implementing manufacturing, simulations or an extension framework here.

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

**Current evaluation:** the reproducible ACS0.1.19 spike established a viable
adapter, including auxiliary-contact ellipse tangency, native/WASM agreement and
the measured 100-entity workloads. ADR0025 records its provisional selection;
`solver-spike.md` includes difficult-case latency rather than claiming universal
16 ms performance. The production core passes 32 tests, including eleven
independent acceptance cases, and native/WASM compile and lint checks. See
`acceptance-core-report.md` and `review-core-architecture.md`. Actual production
WASM execution, interactive CPU measurements and full document/UI acceptance
remain required. ADR0026 governs document integration; the source-scaling
decision remains pending. These milestones do not reduce the accepted scope.

## Out of scope

- 3D sketches, assemblies, parametric 3D parts and a general CAD kernel.
- Automatic constraint inference while drawing; snapping assists placement but
  does not silently add constraints.
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

No product-scope question remains open. Solver selection and model decisions
remain with their documented evaluation/ADR process; there is no default
presumption in favour of an own Newton/LM solver.

## Links

Requirements: R-EDIT-042
Depends on: `0049-linked-clone`, `0016-boolean-operations` (kernel tolerance)
Related: `0070-measure-and-dimension` (shared dimension foundation), `0057-clone-overrides`, `0060-snapping` (sketch targets), `0063-manufacturing-workbenches`
ADRs: `docs/adr/0025-sketch-core-and-solver.md`, `docs/adr/0026-parametric-sketch-nodes-and-workbench.md`; source scaling uses the `needs-customer` process
PR: -
