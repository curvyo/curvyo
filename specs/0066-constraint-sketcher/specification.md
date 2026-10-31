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
2. Given a valid entity selection, then coincident, horizontal, vertical, parallel, perpendicular, tangent, equal, fixed, symmetric and midpoint are discoverable named constraint tools in the existing **left tool rail**, with settings in the existing single top tool-options bar. Sketches Properties inspects existing relationships rather than presenting a wall of creation actions. ~~Settings in the existing single top tool-options bar.~~ *(Superseded in part by AC24 and AC34, revision 2, 2026-10-31: the rail is the one home of constraint tools; no tool-options bar for Sketch tools where avoidable.)* Tool eligibility and invalid selections are explicit. Applying a valid action adds that relationship and updates the solved result.
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
13. Given Polyline, then successive clicks create a continuous connected chain of lines and arcs with shared endpoints, without returning to a new unrelated start after each segment. ~~Alt changes the applicable mode once per key press without losing pending input.~~ *(Superseded by AC25 and AC34 pending Q1: the M key cycles the mode once per press without losing pending input, including for the first segment.)* Modes cover free line, perpendicular line, tangent line, tangent arc and left/right perpendicular arcs, with angle available as numeric direction. Clicking an eligible existing open line/arc endpoint resumes from that endpoint without a near-coincident duplicate. Clicking the chain start closes it. Enter finishes open when not confirming a numeric step; Escape or right-click discards the live unfinished step and ends continuation, retaining committed segments. Open and closed chains remain constrainable; selected mode intent produces the corresponding valid relationships. The standalone Arc tool also continues from an explicitly picked open line/arc endpoint in its start-first construction, reusing that endpoint identity; a centre-first variant keeps its explicit centre role and offers a discoverable start-first continuation route.
14. Given Circle or Ellipse, then construction variants are explicitly named ~~in the existing floating tool-options bar~~ *(superseded by AC35 pending Q3: variants cycle with M inside one rail tool and the active variant is named by the on-canvas label)*. Circle supports centre/radius and three-point construction; Ellipse supports centre/axes and axis-endpoints/third-point construction. Previews identify the centre, axes or construction picks. Degenerate input refuses completion without discarding earlier picks needed to correct it. The result remains analytic, constrainable geometry; choosing a construction method alone does not silently create persistent external-reference constraints.
15. Given Slot, then the maker picks its two centreline ends and width to create a straight slot with connected tangent line/arc boundaries. Width and centre spacing are editable, constrainable values. Curved slots are outside this feature.
16. Given a valid corner, then Fillet previews a radius-controlled rounding while preserving applicable constraints and dimensions through explicit entity-reference remapping. If the rounding cannot preserve existing relationships, it identifies affected constraints in Sketches and leaves the sketch unchanged; it never silently drops constraints. Cancellation restores the original corner; commit and undo preserve the associated topology and relationships as one edit.
17. Given constraint inspection, then selecting a constraint locates its entities and selecting geometry allows inspection of remaining freedom or the relationships fixing it. Sketches contains the corresponding status, DOF and diagnostic controls; the canvas highlights the geometry and annotations. Conflict, redundancy and numerical failure remain distinguishable without colour. Exact per-entity classification and conflict diagnostics must be verified in solver evaluation.
18. Given the Sketch workbench, then the existing canvas, rulers, Properties tab strip and floating tool cards remain the application shell. The selector above Properties identifies **Vector / Sketch**; **Sketches** identifies the active page sketch and inspects existing constraints, dimensions and all state/diagnosis; named parameters/formulas live in a separate **Parameters** tab (fx). The left floating tools change with workbench; ~~construction variants and active-tool settings use the existing single floating top bar~~ *(superseded by AC34/AC35: Sketch tools show no tool-options bar where avoidable; modes and variants are cycled with M and named by an on-canvas label)*. There is no additional app header, breadcrumb/Finish bar, second constraint toolbar or bottom state bar. Ordinary Bézier Pen, Star and Polygon are not copied into Sketch by default. Active editing scope and the route back to Vector are explicit. Future work contexts do not authorize implementing manufacturing, simulations or an extension framework here.

19. Given a page, then Sketch workbench directly edits that page's one owning Sketch object, without New sketch or an active-sketch picker. Changing page changes editing scope and cancels only unfinished input. Navigating existing workbench/page contexts creates no history step. Creating a dedicated source page from a populated Vector page is a separate reversible document operation under Proposal A in the revision plan, preserving all existing content. The first durable graph edit can atomically materialize an absent page sketch, including defining a parameter before drawing geometry. Save/reopen, undo/redo, deletion/recreation and concurrent first creation preserve the singleton and stable references. Linked clones remain derived objects under AC9, not additional editable sketch systems. Exact copy/paste/clone reconciliation belongs to ADR0026 and must not silently merge or discard graph intent.
20. Given Sketch construction, then Tab enters its numeric entry *(placement superseded by AC33: the entry is anchored at the object on the canvas, not in a separate bar; all other behavior of this criterion stays)* with the relevant value selected and Tab/Shift+Tab cycle the visible fields (line length/angle, circle/arc radius/angle as applicable, rectangle width/height). Enter confirms the valid current step; invalid input retains focus and last valid geometry. Escape/right-click never commits geometry. Escape works with canvas, numeric draft, drag and all multi-pick phases; completed operations remain. A subsequent Escape returns to Select. Right-button press/release/contextmenu cannot masquerade as a left click or cancel twice; idle context-menu behavior remains.
21. Given Sketch, then Vector snapping settings, targets, axis masks and held-Alt inversion do not affect Sketch drawing or moving. Automatic inference is limited to local sketch axes: a proposed axis relationship is visible before commit and becomes an inspectable constraint on confirmation (X axis fixes local y = 0; Y axis fixes x = 0; their intersection fixes both). Explicit endpoint connectivity/closure and tangent/perpendicular mode intent are retained, not treated as general snapping. ~~Alt changes modes in drawing context~~ *(superseded pending Q1: M changes modes; Alt then has no drawing-mode meaning)*; established Alt selection behavior and native text/system chords remain context-scoped.
22. Given selection, then point/endpoint, analytic edge and dimension/constraint are distinct selectable targets with consistent hover/selection feedback. Shift toggles membership; marquee containment/crossing and Ctrl removal follow existing Curvyo conventions. Tied overlapping targets can be disambiguated without accidental dragging. Constraint tools accept preselection or collect eligible references and do not silently ignore incompatible members. Selecting a relationship highlights its participants; selected geometry can be inspected. Selection is preserved across successful compatible edits and cleared only when its targets cease to exist or the user changes it.
23. Given the dimension tool and selected geometry, then it offers the relevant length, distance, angle or radius/diameter ~~in the existing tool-options bar~~ *(superseded by AC34: the kind follows the selection and alternatives cycle with M, labelled on canvas)*; alternatively it collects references interactively. A circle/arc selection makes radius editing directly discoverable. The maker positions the annotation and enters a value/expression using the existing numeric-entry style; reference output remains read-only. Existing dimensions edit their expression without adding a duplicate relationship. Constraint creation stays in the existing tool rail and single top bar; no additional floating constraint toolbar is introduced.

## Customer revision 2 (2026-10-31)

Origin: Customer feedback after trying the revision build. Items marked *Proposal*
are product-owner details the customer has not accepted yet. AC6 and AC13 are
extended, not replaced, by AC25-AC29. The angle stops in AC28 are the product's
one table (multiples of 15 and of 22.5 degrees, `curvyo-ui-core/src/angle_snap.rs`),
as the Pen uses them (R-EDIT-047, `0072`). Field reference: FreeCAD Sketcher shows
dimensions and constraint symbols permanently on the canvas; SolveSpace does the
same; LightBurn has no constraints. We do better by keeping the numeric entry at
the object, one key (M) for every mode and variant, and no options bar in Sketch.

24. Given the Sketch workbench, then every constraint tool of AC2 (coincident, horizontal, vertical, parallel, perpendicular, tangent, equal, fixed, symmetric, midpoint) and the dimension tool appear as buttons in the left tool rail, grouped as draw, constrain and dimension, each with a tooltip naming the tool and its shortcut. Choosing one with a valid preselection applies it as in AC2. No constraint can only be created from Properties.
25. Given a polyline with its start point placed and no segment committed, when the maker presses M, then the mode of the first segment changes to the next applicable mode, the preview and the on-canvas label update at once, and pending numeric input is kept. Cycling works identically for the first, second and every later segment; a test cycles all applicable modes on segment 1.
26. Given a polyline of any length (tested at segment 1, 2, 3, 5 and 8) in tangent-arc or perpendicular-arc mode, when the cursor moves to either side of the tangent or start direction, then an arc preview is always visible: the inner side produces a concave arc (opposite sweep to the outer side) and the outer side a convex one, and the preview switches sides continuously with no frame without an arc. Only when the cursor lies on the tangent line itself is the arc degenerate; the preview then shows a straight segment marked as degenerate and refuses commit.
27. Given line, polyline or arc construction and the cursor within the capture radius of an existing open endpoint, or of the chain start, then the point under construction is pulled onto that endpoint and a capture marker (ring plus "Endpoint" label) shows on it before the click; outside the radius there is no pull. Commit reuses the existing endpoint identity (no duplicate near-coincident point; closing a chain yields one shared point). The radius is 10 screen pixels at any zoom (*Proposal*). The pull ignores all Vector snapping settings; when several endpoints are in range, the nearest wins.
28. Given a Sketch drawing step with a free direction (free line, polyline free-line mode, slot centreline), when Ctrl is held, then the direction from the segment start to the pointer snaps to the nearest angle stop, measured from the sketch local X axis, keeping the pointer distance as length; the angle shows in the live readout; releasing Ctrl restores the free direction. Ctrl has no effect in tangent or perpendicular modes, whose direction is fixed by the mode. When an endpoint capture (AC27) is in range, the capture wins. Default (Q2): the lock only positions points and adds no constraint.
29. Given unconstrained geometry of every entity kind (point, line, circle, arc, ellipse, rectangle and slot members), when the maker drags it with the mouse in Select, then it moves with the pointer: a point moves itself; a line, circle, arc or ellipse dragged by its edge translates as a whole, an endpoint or centre moves only that point; remaining geometry follows hard constraints, stops at the constraint limit as in AC6, and fixed geometry does not move. The 16 ms budget of AC6 applies to each kind. One drag is one history step; Escape mid-drag restores the start state. Tests drag each kind, including an arc and a circle, before and after adding a constraint.
30. Given Select in Sketch, when the pointer hovers geometry that can be dragged, then the cursor is an open hand; while a drag is in progress it is a closed hand; over fully fixed geometry, over non-draggable annotations or during a drawing tool it is the tool's normal cursor. The cursor state is asserted in the actual event path for hover, press, drag, release and Escape.
31. Given driving or reference dimensions, then each stays permanently visible on the canvas in the Sketch workbench (no selection or hover needed) as an annotation: linear (length or distance) with two extension lines, a dimension line with arrowheads and the value text on or beside it ("50 mm"); radius with a leader from the arc or circle ("R 25 mm"); diameter ("Diameter" symbol, "50 mm"); angle with an arc between the lines and arrowheads ("45 deg"). Text uses the display unit and shows the resolved value of an expression. A reference dimension is distinguishable without colour (parentheses and a dashed line, "(50 mm)") from a driving one. Text keeps constant screen size at every zoom; label position is draggable and persists (existing dimension label position). Hover or selection highlights the annotation and the dimensioned entities; double-click edits as in AC3. Annotations are never output geometry (AC8).
32. Given geometric constraints, then each shows a small symbol next to the constrained geometry: horizontal, vertical, equal, tangent, coincident, perpendicular, parallel, fixed, symmetric, midpoint and the axis relations of AC21. Symbols keep constant screen size, do not overlap each other (stacked or offset when several share a place), carry an accessible name, and selecting one selects that constraint and highlights its participants (AC22). A conflicting or redundant constraint's symbol also carries the AC5 non-colour marker. The icon source and its licence are recorded in the repository attribution before the story is accepted (lead). *Proposal:* a display toggle for symbols in Sketches, shown by default, to declutter dense sketches.
33. Given Tab or a dimension edit, then the numeric entry fields appear on the canvas directly at the object being drawn or edited, in the existing Curvyo entry style (`0052`): line length near the segment midpoint and angle near its start, circle or arc radius at the radius leader, rectangle width and height at the matching edges, slot width at its edge, fillet radius at the corner, an edited dimension at its text. Fields stay inside the visible canvas (they move to the other side near an edge), do not cover the live cursor point, follow pan and zoom, and keep AC20's Tab, Enter and Escape behavior. There is no separate numeric bar.
34. Given any Sketch tool, then there is no polyline mode dropdown and no tool-options bar for Sketch tools unless a tool has a setting that cannot be expressed by M or an on-canvas field (such a bar is named in the PR). Pressing M (Q1) cycles the drawing mode once per key press (key repeat ignored) among the applicable modes of AC13 and, for the dimension tool, among the dimension kinds the selection allows. A small label at the start of the current segment names what is being drawn ("Line", "Perpendicular line", "Tangent line", "Tangent arc", "Arc left", "Arc right"), updates on each press and disappears on commit or cancel. M typed into a numeric or text field is text, not a mode change. The label and the mode add no history step and are no output geometry. *Proposal:* Shift+M cycles backwards.
35. Given the Arc, Circle and Ellipse tools (Q3), then their variants (Circle centre/radius and three-point; Arc start-first and centre-first; Ellipse centre/axes and axis-endpoints/third-point) are cycled with M inside the one rail tool, the active variant is named by the on-canvas label, and the rail has no separate variant buttons. M is available before the first pick; after a pick it shows a hint to finish or press Escape to change the method, and earlier picks are never discarded silently (AC14).

### Superseded criteria

- AC13: the Alt mode key (replaced by M, AC25/AC34, pending Q1); the rest stands.
- AC14: variants in the tool-options bar (replaced by AC35, pending Q3).
- AC18: "one existing floating tool-options bar" for Sketch (replaced by AC34/AC35).
- AC20: the placement of the numeric entry (replaced by AC33); Tab, Enter, Escape stand.
- AC21: "Alt changes modes" (pending Q1); axis-only inference and no Vector snapping stand.
- AC2 and AC23: settings and dimension kind in the options bar (replaced by AC24/AC34).
- AC6: extended by AC29 to all entity kinds; the performance and limit rules stand.

### Open questions for the customer

- **Q1 Drawing-mode key.** A (default): M replaces Alt; Alt no longer cycles drawing modes. B: both M and Alt cycle. Recommendation: A, one key avoids the window-manager Alt clashes of `0034`.
- **Q2 Ctrl angle lock.** A (default): it only positions points, no constraint is added. B: 0 and 90 degrees add Horizontal and Vertical, visible before commit like axis inference. Recommendation: A, so the lock never hides a surprise relationship.
- **Q3 Arc and Circle variants.** A (default): variants cycle with M inside one rail tool, no options bar. B: separate rail buttons per variant. Recommendation: A.

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

- **SKETCH-REVISION-2:** customer feedback of 2026-10-31 (F1-F11) is recorded as
  AC24-AC35; Q1-Q3 await the customer and have safe defaults.

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
