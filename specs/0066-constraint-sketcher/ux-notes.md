# Constraint sketcher — revised UX study

Status: Production source/artifact UX review APPROVE; independent browser acceptance and full gate passed. Customer story acceptance remains pending.
Authority: latest customer screenshot correction relayed by the lead. Sketch is its own
workbench **inside the existing Curvyo UI**, not a new application frame. The customer
accepted named parameters/formulas and the revised static demo, and requested implementation.
The customer rejected the bottom Demo state bar: all sketch status and diagnostics belong
inside the Sketches Properties tab, never in an extra bottom area or the app status bar.
The previous app header, breadcrumb/Finish controls and separate constraint bar are obsolete.
Interaction details not explicitly requested below remain **(Proposal)**.

Visual artifacts: [demo](demo/index.html), [parameter view](demo/sketcher.png),
[conflict view](demo/sketcher-conflict.png). These are scripted UI studies, not a solver or
application integration; screenshots may show an earlier revision until regenerated.

## Existing shell, workbench-specific content

```text
+--+---------------------------------------------------+-------------------+
|mm|                 existing horizontal ruler         | [Sketch        v] |
+--+---------------------------------------------------+--+----------------+
|  | [active tool options / construction variant]       |  | Sketches       |
|r |                                                   |t | [Bracket     v]|
|u | [floating tool card]                               |a | 3 DOF  [Show]  |
|l |   Select / Point                                  |b | Constraints    |
|e |   Polyline / Arc        sketch geometry            |s | Dimensions     |
|r |   Circle / Ellipse      origin + dimensions        |  | Parameters     |
|  |   Rect. / Slot                                    |  |                |
|  |   Fillet                                          |  |                |
+--+---------------------------------------------------+--+----------------+
| existing coordinates / status / zoom / units                             |
+-------------------------------------------------------------------------+
```

Keep the current canvas rulers, floating left tool cards, one floating top tool-options
bar and existing right Properties panel with its vertical tab strip. No app header,
permanent docked sketch rail, alternate framing or second constraint toolbar. Reuse existing
tokens, sizes, focus rings, panel collapse/scroll rules and tool-options overflow pattern.
Switching tools changes content in place and never resizes the canvas.

The workbench selector sits **above the right Properties panel**, with current **Vector**
and new **Sketch** choices. It identifies work context, not an active drawing tool. Sketch
changes the tool set; its active tool's variants/settings occupy the existing top floating
bar. Constraints, diagnostics, dimensions and parameters live in the **Sketches** Properties
tab. Any constraint shortcuts shown for selection belong in that same tool-options bar or
Sketches tab, never a second floating strip. No normal Bézier Pen, Star or Polygon by default.

**(Proposal)** expanded Properties shows the full Vector/Sketch choice above its body/strip.
Collapsed Properties keeps its existing 32px strip (coarse-pointer width as today) and shows
a compact native V/S choice at the top, accessible name `Workbench`, with Vector/Sketch
as full accessible option names. The collapsed choice does not expand the panel or add a
full-width header; keyboard/native selection behaviour remains available. Keep the current
collapse control, tab access and camera-origin preservation semantics. Switching workbench
preserves the user's collapsed/expanded preference; selecting a Properties tab opens it by
the existing rule. A tooltip may name the current workbench; no permanent hint is added.

## Scope, navigation and undo

Selecting Sketch enters/re-enters the active sketch work context. **(Proposal)** a sketch
picker plus New/Edit actions lives inside Sketches, showing the current sketch name;
no separate view breadcrumb or Finish button. Selecting Vector exits the work context,
restores its drawing tool set and selects the sketch object. Workbench navigation itself
is not a document edit or a giant undo transaction. Already committed sketch edits remain.
**(Proposal)** switching workbench cancels only the unfinished gesture/entry and preserves
completed operations; Escape first cancels that gesture, then clears selection.

Geometry is selected as sketch points, segments, arcs and analytic curves, not Bézier
handles. Other drawing geometry is reference context only; it cannot accidentally join the
sketch. External-reference constraints require an agreed rule. Keep local origin/axes
visible. Unresolved constraints retain the last valid outline and their diagnosed state
when returning to Vector; do not silently discard constraints or claim successful solving.

## Tools and construction variants

Make variants explicit named choices in the existing floating options bar, keyboard
reachable and discoverable without modifier-only gestures. Exact ellipse/slot variants
remain proposals; the customer requested these construction families:

- Polyline: successive connected straight segments; Enter finishes open, clicking the
  start closes, Escape removes the unfinished segment. Shared endpoints are structural.
- Circle: Centre/radius and Three points. Preview construction points and analytic circle;
  collinear three-point input refuses completion while preserving earlier picks to correct.
- Ellipse: proposed Centre/axes and Axis endpoints/third point; show centre, axes, perimeter.
- Slot: proposed Centreline/width, two spine ends then width; retain parallel sides and
  tangent semicircular caps as intended relationships. Curved slots are not implicit scope.
- Rectangle/Arc: sketch-specific construction options rather than ordinary vector controls.
- Fillet: pick corner/adjoining entities, enter radius, preview trimming/tangency. Retain
  unaffected constraints and show affected references before commit. If they cannot be
  retained/rebound, keep the last valid sketch and explain the affected constraints;
  never silently delete them. Escape restores the corner; commit is one history step.

Selection links geometry, dimension annotations and constraint rows. Shift extends it.
Applicable constraints are named actions; incompatible actions stay focusable/dimmed with
reasons in tooltips. Snapping assists placement without silently creating constraints.

## Sketches tab: dimensions, parameters and diagnosis

Driving dimensions and reference measurements are visibly distinct. Double-click edits,
with a panel keyboard equivalent. Enter commits; Escape restores; invalid units/values
leave geometry unchanged. Inputs retain native/local undo; elsewhere use document undo.
Named parameters and formulas are accepted scope: show names, units, values and expressions,
with dependent dimensions identifiable. Reject unresolved names, cycles and invalid edits
without replacing the last valid result. Exact parameter-table interaction remains proposed.

Show Under-constrained with DOF count, Fully constrained (0 DOF), Redundant, Conflicting
and Solver failure as distinct text states, not colour alone. Show/Inspect reveals free
geometry and allowed motion or the constraints fixing selected geometry. Diagnostic rows
select/highlight implicated constraints and geometry. Conflicts offer explicit edit/delete;
never imply a minimal conflict set without solver evidence. Conflicting/failed states show
DOF unavailable if no valid count exists, not a misleading zero. Repairs are undoable.
All these counts, states, inspection and repair controls stay inside Sketches. Canvas
constraint marks/selection highlights remain geometric feedback; they do not require a
second diagnostics area. Static demo-state controls, if retained for the study, live inside
Sketches only and are not product controls. Do not add them to the app status bar.

## Visual-study boundary

Future manufacturing/extension/laser-preview workbenches use this same shell and selector;
read-only previews identify Preview plus machine/job and expose inspection, not sketch edits.
This is a consistency direction, not authorization for a generic framework or plugin API.
The static demo must identify scripted states/no solver inside Sketches. Keyboard, focus, geometry,
constraint retention, save/re-enter and performance need later implementation verification.
Customer implementation authorization is recorded above; no solver feasibility, test result
or production UX acceptance is claimed here. Those require the implemented sketcher.

## Production source and saved-layout review

Verdict: **APPROVE for reviewed source wiring and saved layout**, not final interaction
acceptance. Reviewed the refreshed actual `target/0066-browser/plate.png` (1440×1000),
bounded Sketch components and existing-shell wiring, and `frontend-handoff.md`.
The screenshot shows the accepted workbench selector, own left tools, one options bar,
Sketches diagnostics and parameter editing without a new header or bottom state area.
Collapsed Geometry retains every entity through native disclosure; compact relationship
rows keep status, dimensions and the parameter editor readable together in this viewport.
Selected geometry, selected relation and dimensional annotations remain distinguishable.
The displayed 5 DOF is reported application state, not independently solver-validated here.

Source inspection confirms active-sketch draft isolation, native input editing, cancellation
and focus paths, accessible input/error associations, full constraint names and unbounded
geometry disclosure. Earlier source/layout findings are addressed; no remaining concrete
blocker was found in these artifacts. The producer reports the tracked browser workflow
passed focus, Escape, draft isolation, conversion and collapsed workbench switching; this
reviewer did not rerun it. Independent extended browser acceptance and the full gate remain
pending with the tester. Other viewport sizes, themes, conflict states and live keyboard
interactions are not established by this single screenshot. No production edits, builds,
tests or live browser operations were performed for this review.

### Latest reference-measurement artifact review

Verdict: **CHANGES REQUESTED for reference-measurement presentation**; the existing-shell
layout remains approved. The latest `remaining-reference-reopened.png` shows a measured
diameter annotation of 21.17 mm (ref), but its selected Reference row shows an editable
99 mm expression instead of the resolved measurement. Bounded source inspection confirms
`SketchesPanel` renders the expression editor for an edited reference row without retaining
its measured output. This contradicts the interaction map's read-only resolved Reference
measurement contract. Show the resolved value read-only in Reference mode; retain the
explicit mode checkbox and reserve driving-expression editing for driving mode.

The refreshed `plate.png` still follows the accepted selector, left tool rail, single
options bar and Sketches-only diagnostics layout, with no bottom state strip. Source focus
and native-input handling retain their earlier reviewed paths; no additional concrete
focus blocker was identified. Tester-reported B1/B2 creation, numeric, invalid Fillet preview
and save/open keyboard checks passed; B3/B4/B5 were ongoing at review time. These are
attributed results, not tests run by this reviewer. This review makes no performance PASS
claim and does not replace independent browser acceptance or the full gate.

Reference-mode keyboard contract: row Enter, annotation Enter or Edit selects/inspects the
relationship and makes its existing Reference checkbox reachable/focused; none implicitly
changes the mode or exposes a writable measured value. Space on that checkbox explicitly
switches modes through the domain command. In driving mode the normal expression entry
supports native typing/undo, Enter commit and Escape draft discard. Turning Reference on
replaces that entry with the resolved read-only output, retaining a surviving focus target.
Tab navigation must reach the mode checkbox and row actions without an editable reference
value. Mode changes remain undoable; no new hint label or extra control is required.

Source recheck: the producer now guards expression editing with `!row.reference` and keeps
the resolved output and existing checkbox in Reference mode. This addresses the identified
rendering cause. A panel value such as 21.166667 mm and a 21.17 mm annotation represent valid
different display precision; the stored nominal 99 mm must not replace the measured output.
Final closure still awaits the refreshed artifact and independent browser evidence;
CHANGES REQUESTED remains the recorded review status until those arrive.

### Reference correction: final source/artifact verdict

**APPROVE for UX source and saved-artifact review.** The refreshed production
`remaining-reference-reopened.png` now shows read-only 21.166667 mm beside the checked
Reference control, consistent with the 21.17 mm (ref) canvas annotation. The checkbox has a
visible focus indicator; the misleading editable nominal 99 mm field is absent. This closes
the identified presentation blocker and supersedes the prior CHANGES REQUESTED verdict.
The accepted existing shell remains intact, including Sketches-only diagnostics and no
extra bottom state strip. No further concrete UX blocker was identified in reviewed evidence.

The producer reports its strict actual-browser reference workflow passed; independent
tester repetition is still pending at this review. This approval concerns inspected source
and saved presentation, not independent interaction, performance or full-gate acceptance.
No browser, build, test or production-source operation was performed by this reviewer.
