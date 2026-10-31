# Constraint sketcher — revised UX study

Status: Customer requested interaction/UI revision after trying PR #228. Earlier production
source/artifact review and gate results remain historical checks, not customer acceptance.
Customer now authorizes revision implementation; revised product acceptance remains pending.
See [revision plan](revision-ux-plan.md), especially its finite mode/inference handoff.
Authority: latest customer screenshot correction relayed by the lead. Sketch is its own
workbench **inside the existing Curvyo UI**, not a new application frame. The customer
accepted named parameters/formulas and the revised static demo, and requested implementation.
The customer rejected the bottom Demo state bar: all sketch status and diagnostics belong
inside the Sketches Properties tab, never in an extra bottom area or the app status bar.
The previous app header, breadcrumb/Finish controls and separate constraint bar are obsolete.
Interaction details not explicitly requested below remain **(Proposal)**.

## Current revision authority (supersedes older conflicting notes below)

One Sketch object belongs to each page, with its initial local origin at the page centre.
There is no New sketch button or sketch picker. Entering Sketch edits that page's sketch;
human-readable object naming replaces generated suffixes. Sketch has no snapping: only
axis inference (x = 0 / y = 0), structural endpoint connectivity and explicitly requested
mode relations. Alt cycles active construction modes once per press without losing the
pending chain; Select Alt retains its existing behavior. Tab enters/cycles numeric input.
The exact mode cycle, eligibility, inference and numeric cancellation rules are recorded
in the revision plan's handoff table for implementation.

Standalone Arc has two proposed variants: Three points (Start → Through → End) and
Centre, start, end (Centre → Start → End direction). See the revision plan's finite Arc
table for radius applicability and cancellation. Mode switching preserves pending spatial
picks and explicitly relabels their roles; it never moves them or committed geometry.
Radius is editable only after Centre and before Start; otherwise it is derived. Invalid
final picks retain the earlier picks for correction rather than silently resetting them.

Use existing styled workbench menus, left geometry/constraint tool groups and the single
top options bar. Sketches inspects existing constraints/status; named parameters/formulas
have a separate fx tab. No new header, extra toolbar or bottom state/diagnostic surface.
`demo/index.html` is the revised static study, with scripted line/arc continuation, radius,
Tab, Alt and cancellation scenarios; its limitation and scenario controls appear only
inside Properties. This study is not a solver, a complete tool implementation, or customer
acceptance. Root owns browser capture and review; no browser/build check claimed here.

Static revision follow-up: the lead subsequently delegated exclusive demo-browser review.
Chromium check `target/0066-browser/ux-demo-check.cjs` passed the declared scripted chain,
Alt/Tab/Shift+Tab, Escape/right-click, endpoint resume, radius, menu/focus, collapse and fx
flows without page errors, plus 1000×650 overflow check. Screenshot review confirms the
lean shell and separate parameter pane; Page 1 and Add parameter icon were polished.
This supersedes only the no-demo-browser statement above. Production implementation,
solver behavior and customer acceptance remain unverified by this static check.

### Revised production frontend: initial source review

Bounded source review found the intended styled workbench primitive, no New/picker UI,
constraint-tool entry points in the left rail, one contextual options bar, lean existing-
relationship inspector and separate parameter tab. This is not a browser/layout verdict.
Reported fixes: collapsed workbench menu needs end alignment to remain inside the viewport;
full Arc/Polyline variant names must fit so left/right modes remain distinguishable.
The producer reports both addressed in source; runtime evidence remains pending.

Further items sent to the producer: numeric failures need a mounted, visible associated
error even when Properties is collapsed or another tab is active; mode switches must not
silently reinterpret a Length draft as Radius; the proposed discoverable selected-endpoint
Resume action is not yet present in the reviewed toolbar/inspector. Canvas Tab routing,
numeric Tab/Shift+Tab and draft Escape paths exist in source. Numeric prefill is still
being completed by the producer. No production edits, builds or browser runs were made
for this review, and no revised production UX approval is claimed.

Current visual artifacts: [demo](demo/index.html), [Sketch view](demo/sketcher.png),
[Parameters view](demo/sketcher-parameters.png), [compact view](demo/sketcher-compact.png).
These are scripted UI studies, not a solver or application integration. The older
`sketcher-conflict.png` is a historical study, not the revised product layout.

## Existing shell, workbench-specific content

### Revision review checkpoint (2026-10-30)

**CHANGES REQUESTED for the reviewed parameter controls; live UI review pending.**
This checkpoint reviews current source and the refreshed `revision-host.png` and
`revision-dimension.png` production artifacts. It does not replace the historical
source/artifact verdicts below with an interaction approval.

1. `SketchParameters.tsx` uses native unit selects in both existing parameter rows
   and the Add parameter form. This violates the design system's explicit
   no-popups-in-Properties rule. Use the existing inline segmented-choice pattern
   for Length (mm), Angle (rad) and Scalar, with one keyboard group and named
   selected state; retain the same unit semantics and commit/cancel behavior.
2. The Workbench and Arc triggers in the artifacts resemble bordered native fields.
   Current source proves they are custom `TracePresetMenu` buttons; appearance is
   not evidence of a stale build. Their open menus, surrounding surface, focus and
   compact alignment still need direct comparison with Vector controls and the
   accepted study. Component reuse alone is not visual acceptance.

The saved artifacts preserve the existing rulers, workbench-specific left rail,
one options bar, separate fx tab and Properties-only diagnosis, without New/picker
or extra bottom chrome. Local `ui-ux-pro-max` search for keyboard focus, Escape and
compact semantics reinforced visible focus and correct control state; the existing
desktop design system takes precedence over generic web/mobile guidance. No browser,
build, test or product-source operation was performed for this checkpoint.

### Revision live-browser review (2026-10-30)

**CHANGES REQUESTED: two bounded presentation fixes.** The exclusively owned browser
review ran `target/0066-browser/ux-revision/check.cjs` against the fresh production
preview at port 1421. Captures and `report.json` are in that script's directory.
This is a UX check, not a repeat of solver or independent feature acceptance.

1. Retain the parameter-unit finding above: the actual Add parameter form exposes
   one native combobox. Replace both add/existing-row unit selects with inline
   segmented choices, using the existing panel choice/focus pattern.
2. Give the Workbench selector its chrome surface. Its surrounding unstyled flex
   column currently exposes the white application background, making a flush white
   field/strip above the grey panel. The accepted study uses a grey inset strip.
   Wrap only this selector in `--toolbar-bg`, with the study's 4px vertical/7px
   horizontal inset when expanded; keep compact horizontal padding zero and the
   existing collapsed width. Preserve the custom menu behavior and menu tokens.

The actual Workbench and Arc open menus use the expected grey chrome, selected mark,
blue focus ring and readable full names. Arc menus visually match the existing
`TracePresetMenu` primitive in Vector's Trace bar. Escape returns focus to the Arc
trigger. Keyboard Enter/End/Enter selects Sketch. At 1000×650, the Arc menu and top
bar fit; the collapsed Workbench menu bounds are x=768, width=232, ending exactly at
the viewport edge. No document horizontal overflow or page errors occurred. The
single options bar, existing left rail, right tab strip and fx tab remain intact.
Only the supported current light theme was reviewed; no invented dark-theme pass.
No production code or builds were changed. Browser closed and ownership released.

### Revision presentation fixes: closure

**APPROVE for the two bounded presentation corrections.** Passive re-review of
current `SketchParameters.tsx`, `PropertiesPanel.tsx` and the refreshed
`fixed-workbench.png`, `fixed-parameter-form.png`, `fixed-parameter-row.png` and
`fixed-collapsed.png` closes both findings above. Add and existing parameter rows
now use the established inline `ToggleGroup` for mm/rad/Scalar, with visible blue
selected state. The Workbench selector now sits on the panel's grey chrome with
the requested inset. At 1000×650 collapsed, S and its chevron are visible in the
32px strip and the open menu stays within the viewport. Scoped compact padding
avoids changing the existing menu primitive for other users.

The producer reports keyboard unit creation/editing and menu/focus checks passed;
this closure independently inspects source and refreshed rendered artifacts,
without rerunning that producer interaction check. It complements this reviewer's
earlier actual-browser keyboard/menu/compact review. No remaining concrete UX
blocker was identified in the reviewed scope; independent feature acceptance,
runtime parity/performance, the final gate and customer acceptance retain their
own evidence requirements. No browser/build/product changes were performed here.

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
