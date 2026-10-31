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

## Revision 2 (2026-10-31)

Source: customer feedback of 2026-10-31 (constraint icon tools, M mode cycle, on-canvas entry,
magnetic capture, Ctrl lock, cursors, FreeCAD-like dimensions and constraint symbols).
**This section supersedes every older line that conflicts with it**: the single top options
bar in Sketch, the Polyline/Arc/variant menus, the "Constraint tool" menu, the Alt mode cycle
and the Resume action in the bar. Not superseded: the Sketches Properties tab as the only home
of status and diagnostics, no new header, toolbar or footer, the Escape cascade, no Vector
snapping. Existing-code facts used below: `SketchTools.tsx` (rail with 11 buttons,
`SketchToolbar` bar with menus "Polyline mode", "Sketch tool variant", "Constraint tool",
fields, Reference, Apply/Preview), `SketchAnnotations.tsx` (DOM text labels positioned by Rust),
`PenEntryChip.tsx` / `EntryField.tsx` / `readoutPlacement` (the on-canvas entry chip).
Everything marked **(Proposal)** is vetoable at the demo. No browser or build was run.

### 1. Constraint tools in the left rail

The Sketch rail keeps its geometry card (Select, Point, Polyline, Arc, Circle, Ellipse,
Rectangle, Slot, Fillet: 9 buttons) and gets **one new card, "Constraints", in column B** (left
edge 68px, top 12px, 8px right of the geometry card, the Path/Boolean card position of row
"Tool rail architecture"). The current constraints/dimension buttons below a divider in the
geometry card go away. Card: `role="toolbar"`, vertical, name "Constraints", one Tab stop,
Up/Down/Home/End inside (row "Rail keyboard model"); button 40px, icon 20px in a 16 viewBox,
1.5px absolute stroke, `currentColor`; hover `--editor-accent-hover`, active
`--toolbar-icon-active-bg`/`-fg`. Order, with a 1px 25% divider after Fixed and after Midpoint:
Coincident, Horizontal, Vertical, Parallel, Perpendicular, Tangent, Equal, Fixed | Symmetric,
Midpoint | Dimension. 11 buttons, about 500px: fits the 640px minimum height (544px column B
budget). No dropdown on any button; no flyout.

Behaviour (the Vector rule "selection first, else tool", **(Proposal)**):
- Eligible selection exists: a click applies the constraint at once, one history step,
  selection stays for the next one; the button flashes no state.
- Otherwise the click arms the tool (`aria-pressed`); the picks follow the existing point/edge
  pick rules and order-independent eligibility; the constraint is added on the last pick, the
  tool stays armed. Escape (cascade): disarm to Select.
- Not eligible now: the button stays focusable, drawn at 40% (as dimmed rail buttons today),
  `aria-disabled="true"`, tooltip "Horizontal: select a line or two points". Pressing it does
  nothing and shows no notice. Tooltip text = name, plus the reason only when disabled. No
  shortcut letters, no operating hint text.
- Dimension is one smart tool (kind inferred from the picks, as today: length, distance,
  radius/diameter, angle). Its value is typed in the on-canvas entry box (section 3).

**Icons needed.** FreeCAD names are from memory of its `Constraint_*` set; the lead checks
names and the licence before use. Borrowed icons are redrawn single-colour (`currentColor`,
the rail look); FreeCAD's colour versions must not appear. The canvas symbols (section 7) are
**our own simplified drawings** of the same shapes: FreeCAD's 64px colour art is unreadable at
12px. Every icon is used in three places: rail (20px), Sketches rows (16px, left of the
constraint name, replaces text-only rows) and canvas symbol (12px in a 16px badge).

| Icon | FreeCAD source | Rail | Sketches row | Canvas symbol |
|---|---|---|---|---|
| Coincident | `Constraint_PointOnPoint` | yes | yes | yes (not for structural joins) |
| Horizontal | `Constraint_Horizontal` | yes | yes | yes |
| Vertical | `Constraint_Vertical` | yes | yes | yes |
| Parallel | `Constraint_Parallel` | yes | yes | yes |
| Perpendicular | `Constraint_Perpendicular` | yes | yes | yes |
| Tangent | `Constraint_Tangent` | yes | yes | yes |
| Equal | `Constraint_EqualLength` | yes | yes | yes, with pair index |
| Fixed | `Constraint_Lock` | yes | yes | yes |
| Symmetric | `Constraint_Symmetric` | yes | yes | yes |
| Midpoint | none; draw in-house (point on a line, tick at the middle) | yes | yes | yes |
| Dimension | `Constraint_Length` as the generic ruler | yes | rows use the kind glyphs below | no (the dimension itself is the mark) |
| On axis (point on x = 0 / y = 0) | `Constraint_PointOnObject` | no (inference only) | yes | yes |
| Length / Distance, Radius, Diameter, Angle | `Constraint_Length`, `_Radius`, `_Diameter`, `_InternalAngle` | no | yes | no |

### 2. The options bar is gone: mode cycle and the mode label

Sketch shows **no top options bar**. The canvas keeps its full height; switching tools never
resizes anything. Where each old bar item went:

| Old bar item | Now |
|---|---|
| Polyline mode, Sketch tool variant, Constraint tool menus | `M` cycles; the mode label shows the result; constraint tools are the rail card |
| Numeric fields, Apply, Preview | the on-canvas entry box (section 3); Enter commits, no Apply, no Preview button (the canvas always previews) |
| Reference check | row 2 of the entry box when a dimension is typed (section 3) |
| Resume from endpoint | clicking a captured endpoint with Polyline or Arc resumes (section 4); for an explicit start from a selected endpoint a "Resume" row action in Sketches |
| Clear / reset picks | Escape cascade |

**Key `M`**: cycles the active drawing tool's applicable modes forward, `Shift+M` backward,
once per press, no key repeat, wraps, skips modes without a precondition (as the Alt cycle
did), pending picks kept and relabelled as in the revision plan table. It replaces the Alt
cycle (Alt is taken by window managers on Linux). Ignored while a text field or the entry box
has focus and with a modifier other than Shift. Vector's `M` (type a move) is unaffected: it
belongs to Select in Vector, and Sketch Select assigns no `M`. Tools with one mode (Point,
Fillet) ignore it.

**Mode label** **(Proposal)**: a small DOM chip, `pointer-events: none`, `aria-hidden`; text is
the domain's mode name, Title case, at most 22 characters: "Line", "Perpendicular line",
"Tangent arc", "Arc, 3 points", "Circle, centre". Style: the Live transform readout's look:
12px, `--toolbar-bg` ground, `--toolbar-icon` text, 2px 6px padding, 4px radius, no shadow,
sits below the entry box and the readout in z. Placement: top-left corner 10px right of and
below the start point S of the current segment (S = the last committed point, for the first
pick the pointer); if the rubber band leaves S into that quadrant, take the diagonally opposite
quadrant; clamp inside the canvas and clear of the rail (128px, as the entry chips). It follows
S on pan and zoom. Mode change swaps the text in the same frame, no animation. A visually hidden
polite live region says the new mode ("Tangent arc") on each change and once when the tool
opens. Shown: tools with two or more modes, from tool selection while the pointer is on the
canvas. Hidden: Select and constraint tools, tools with one mode, pointer outside the canvas,
during pan or an object drag, after the construction commits or is cancelled (until the next
first pick it returns at the pointer). It stays while the entry box is open.

### 3. Numeric entry box on the canvas

Reuse the Pen entry chip as it exists: surface "Transform entry chip" (`--toolbar-bg`,
`--panel-elevation-shadow`, 8px radius), `EntryField` fields 28px high, 14px tabular, label
inside the left padding (12px), display-unit suffix inside the right edge, 154px / 150px
widths with 72px of text, `--editor-accent` 2px border on the open field, `--field-invalid` 2px
plus a 12px message line under the fields on a refusal. The component stays a Sketch sibling
(design-system rule: a sibling, not a generalisation); share `EntryField` and the placement
helper. Group name = the object kind ("Line", "Arc", "Circle", "Dimension"); field labels name
the quantity ("Length", "Angle", "Radius", "Value"); the fields are the ones the revision plan
gives for the current stage.

- **Opens** with Tab on the canvas when a construction has a pending pick or a dimension is
  selected; also by double-click on a dimension text; focus in the first field, text selected,
  in the opening frame. A drawing tool without a pending pick: Tab does nothing here.
- **Anchored to the object**, not to the pointer: line, arc and dimension, centred on the
  object's midpoint (arc: its mid-arc point) and offset 14px to the side away from the mode
  label and the pointer; circle and radius: right of the perimeter point; rectangle and slot:
  below the far corner; a dimension edit sits over its own text. Clamp, flip and rail
  clearance by the placement rule of the Pen entry chip. Before the first edit it follows
  the object as the pointer moves; after the first edit it stays; it follows the object on
  pan and zoom.
- **Keys unchanged**: Tab / Shift+Tab move between fields (wrap), Enter commits a complete
  valid entry (one history step), Escape closes the box and restores the live pick, the next
  Escape cancels the pending segment (cascade). Focus returns to the canvas after every end.
  The preview runs to the typed values while open. A mode change (`M`) is ignored while the
  box has focus; after Escape it works again and never reinterprets a draft Length as Radius.
- **Refusal**: field border `--field-invalid`, message line inside the box ("Must be more than
  0 mm"), `aria-invalid`, `aria-describedby`; the box stays open. The message lives in the
  box, so it is visible with Properties collapsed; nothing goes to a status area.
- **Reference**: the box of a new or edited dimension has row 2, one checkbox "Reference"
  (14px box, 12px label, `tabindex="-1"` like the Move chip checks, click only, no key assigned). Checked: the value field becomes a
  read-only resolved value, as already specified for Reference rows.
- The Sketches rows keep their editable fields; they are not removed.

### 4. Magnetic endpoint capture

While a drawing tool awaits a pick, an existing sketch point (endpoint, arc/circle centre,
origin) within **12px** screen distance of the pointer captures it: the preview point jumps to
the point's centre, the rubber band ends there, and the capture marker shows. Release at
**16px** (hysteresis, no flicker). Nearest point wins; ties go to the older point. Sketch
points only: no grid, no Vector objects, no invisible targets.
- Marker: the Pen target cue pattern: the point's glyph (10px circle, white fill, 1.5px
  `--toolbar-icon` outline) inside a **18px `--accent` 1.5px ring** with a 1px white casing
  (stronger than the Pen's 20% ring; sketch geometry is thin and dense), instant in and out.
  The matching Sketches row highlights as for hover. Centre and origin use the same marker.
- Meaning: a click on a captured point reuses its identity (structural join; Polyline/Arc
  resume from a free end; closing on the first point previews the closing segment, 1px dashed
  `--accent`). It never adds a tangent or other relation by itself.
- Axis inference stays as specified (axis line `--axis-guide`, proposed mark before commit);
  capture has priority over axis inference.
- Constraint tools and Select use the 8px object hit tolerance, not capture.
- Touch: no capture preview without a hover; the tap uses the 16px hit radius.

### 5. Ctrl locks the direction

Same stops as the Pen: one table (`snap_angle`, multiples of 15 and 22.5 degrees), absolute from
the sketch x-axis, nearest stop wins, length kept. Applies to every pick whose direction from an
anchor is free (Polyline line mode, Rectangle/Slot spine, centre-to-start radius pick, ellipse
axis); tangent and perpendicular modes and captured points ignore it (a capture wins over the
lock, the lock cue then hides). **Visible lock cue while Ctrl is down and the lock is engaged**:
the rubber band is drawn solid 1.5px `--preview-new`; a 1px **dashed (2 on / 2 off)
`--axis-guide`-strength line** runs through the anchor along the locked angle, 24px past the
pointer in both directions (not full viewport: no polar-tracking grid); the live readout chip
shows the locked angle ("30.0 mm, 45°"); the Lock badge of the design system (white ground,
1.5px `--accent` ring, 10px arrow glyph, 16px left of and below the pointer) shows an
angled-arrow glyph. All appear and disappear in the frame Ctrl does (window-level key events,
as the Modifier badge). **Open for the customer:** whether a lock at 0/90/180 degrees also
persists a Horizontal/Vertical constraint, see Frage at the end.

### 6. Cursors

Built-in cursors only (no new custom images; `crosshair` where custom images are ignored).
Rule: the cursor matches what a press would do, unchanged during a press.

| Situation | Cursor |
|---|---|
| Drawing tool, over canvas or captured point | `crosshair` (capture is told by the marker, not the cursor) |
| Select, over a free point, segment, arc, circle (8px hit) | `grab`, with the hover highlight (segment: 4px `--segment-hover` band; point: 18px `--accent-hover` ring) |
| Select, over geometry that cannot move (fully fixed by constraints) | `default`, highlight only |
| Select, drag in progress | `grabbing`; Escape restores, cascade |
| Select, over a dimension text or its dimension line | `grab` (the label can be dragged along its offset); double-click edits |
| Over a constraint symbol | `pointer` (click selects it and its geometry) |
| Constraint tool armed, over eligible geometry | `pointer`; elsewhere `crosshair`; over ineligible geometry `default` |
| Over the entry box or mode label | `text` in a field, else `default` |

### 7. Dimension annotations and constraint symbols

Canvas draws lines, arrowheads, leaders and symbols (render core, screen-constant sizes);
dimension **text stays a DOM label** positioned by Rust (`SketchAnnotations`, keyboard and
screen reader reach; Enter edits). Symbols are canvas marks; their keyboard route is the
Sketches rows (a selected row highlights its symbol). Light theme only, as today.

**New tokens (light theme)**

| Token | Value | Used for |
|---|---|---|
| `--dim-driving` | `#BF2933` | Driving dimension lines, extension lines, arrowheads, text. 4.8:1 on `--canvas-bg`, 5.9:1 on white; 3.0:1 on `--pasteboard-bg` (text carries the 2px halo below) |
| `--dim-reference` | `--panel-muted-fg` (`#5A5A5F`), alias | Reference measurements. 5.6:1 on `--canvas-bg` |
| `--dim-halo` | the document background, 2px | Halo around dimension text and symbols so they read over geometry (like the white "casing") |
| `--sketch-solved` | `#12733A` | Stroke of an entity whose own DOF is 0 (fully constrained, FreeCAD's green). 4.9:1 on `--canvas-bg`. Under-constrained geometry keeps `--toolbar-icon` (`#3A3A3F`). Colour is never the only cue: Sketches states DOF as text |
| `--sketch-symbol-bg` | white at 85% | Ground of a constraint symbol badge |

`--dim-driving` is close to `--destructive` (`#B3261E`); conflicts therefore never rely on hue:
see states. **(Proposal)** all of this is mine to adjust after the first browser look.

**Dimension anatomy** (sizes in screen pixels, constant at every zoom; geometry-attached
positions scale with the sketch)
- Extension lines 1px, start 4px off the geometry point, run 4px past the dimension line.
- Dimension line 1px, between the extension lines; arrowheads filled, 8px long, 6px wide,
  `--dim-driving`.
- Default offset at creation: 24px; next free rank 20px further out when a line already sits
  on that side (ranks never overlap). Dragging the text in Select moves the offset (across)
  and the text position (along); the offset is stored in sketch units.
- Text: 12px, tabular, centred on the dimension line, interrupted by the halo (the line does
  not run under the digits). Format "50 mm" (document display unit), the value as the panel,
  at most the panel's precision, trailing zeros trimmed.
- Linear (horizontal, vertical, aligned): as above. Radius: a leader from the perimeter point
  at the pick direction, one arrowhead on the arc, text at the leader's end outside the circle,
  "R 25 mm"; diameter: a line through the centre with two arrowheads, "⌀ 50 mm". The letter
  names the kind (customer's screenshot shows "25 mm" only; **(Proposal)** to prefix, the
  Placement chip already uses r/⌀). Angle: arc between the two lines with arrowheads, "45°".
- **Driving**: solid `--dim-driving`, plain text. **Reference**: `--dim-reference`, dimension and
  extension lines dashed (4 on / 3 off), text in parentheses "(21.17 mm)". The two differ by
  dash and parentheses, not by colour alone. Formula or named parameter: a 10px "fx" prefix in
  `--panel-muted-fg`; the name is the accessible name, not shown on canvas.
- Small spans: under 56px between the extension lines the arrowheads flip outside (pointing
  in) and the text moves beside the line end; under 24px the arrowheads drop and the dimension
  line becomes two ticks, text only. Under 16px of geometry on screen, dimensions of that
  entity draw only while hovered or selected.

**States.** Hover: a 6px `--accent-hover` band behind line and text box, cursor `grab`.
Selected: every part 1.5px and `--accent`, text in a 1px `--accent` pill; the dimensioned
geometry and its Sketches row highlight with it; Shift extends. Keyboard focus on the text
button: 2px `--editor-accent` outline. Conflicting or redundant: 1px `--destructive` box around
the text plus a "!" prefix, and the Sketches row says it in words. Editing: the entry box
replaces the text. Hit target: text box at least 24px high; lines 8px tolerance.

**Overlap.** Draw order bottom to top: geometry, symbols, reference dimensions, driving
dimensions, hovered, selected. Text vs text: the later text slides along its own dimension line
(up to a third of its length each way) to a free spot; if none, ranks move out. Text over
geometry: the halo keeps it legible. A selected or hovered annotation is always drawn complete
and on top. Over 150 visible annotations, symbols of unselected, unhovered entities are
thinned (see symbols); dimensions are never thinned.

**Constraint symbols**
- Badge 16 x 16px, 3px radius, `--sketch-symbol-bg` ground, 1px `--toolbar-icon` at 40% border,
  12px glyph `--toolbar-icon`, 1.5px stroke. Hover: ground `--accent-hover`. Selected: ground
  `--accent`, glyph white. Conflict/redundant: border `--destructive` 1px and glyph `--destructive`.
  Hit target 16px (padded to 20px).
- Placement: single-entity constraints (Horizontal, Vertical, Fixed on an entity) at the entity's
  midpoint, 12px to the left of its direction; several on one entity form a row along the
  tangent, 2px apart, centred on the midpoint, at most three, then one "+n" badge (opens as
  the full row while the entity is hovered or selected). Point constraints (On axis, Fixed on a
  point, explicit Coincident) 10px up and right of the point. Two-entity constraints (Parallel,
  Perpendicular, Tangent, Symmetric, Midpoint): at the shared point when adjoining (Perpendicular
  as a right-angle badge at the corner, Tangent at the tangent point), else at the middle
  between the two entity midpoints, with 1px dotted leaders to both while hovered or
  selected. Equal: a symbol **on each member**, the "=" glyph with an 8px index digit at its
  lower right ("=1", "=2"); one index per equal group (chains of three equal lines share it),
  numbered in order of the sketch's constraints, renumbered after deletion; hovering or
  selecting one member highlights the whole group.
- Structural joins (shared endpoints of a chain, closure) get **no** symbol; they are one
  point. The origin gets no symbol for its built-in fixed state.
- Symbol colour never carries meaning by hue; the glyph does. Thinning over 150 annotations:
  unselected, unhovered symbols draw as 6px dots at their anchor (type no longer readable)
  until zoomed in or hovered; the Sketches list always has the full rows.

### Design-system and implementation notes

- Add the five tokens and the rows "Sketch constraint card", "Sketch mode label", "Sketch entry
  box", "Endpoint capture marker", "Sketch dimension annotation", "Sketch constraint symbol"
  to `docs/design-system.md` when this section is accepted; not done here.
- Icon licence (FreeCAD, LGPL as I recall) and filenames: the lead verifies; keep the
  borrowed set in one folder with a replace-later note in `docs/technical-debt.md`.
- Review checklist for the implemented UI: rail card keyboard and disabled reasons; `M` and
  Shift+M gated in fields; label placement at four quadrants and canvas edges; entry box at
  800 x 600; capture hysteresis; lock cue appears and disappears with Ctrl on WebKitGTK;
  every cursor row; dimension tiers at 25%, 100%, 400% zoom; 200 annotations; contrast of the
  tokens on `--canvas-bg` and on a changed document background.

Frage 1 (SK-LOCK): Does Ctrl lock at 0, 90 or 180 degrees persist Horizontal/Vertical?
A) Geometry only, never a constraint. B) At 0/90/180 degrees the Horizontal/Vertical symbol
shows in the preview and the constraint persists on click; other stops are geometry only.
Recommendation B (FreeCAD behaviour, the preview shows it before the click, removable in
Sketches). Default if unanswered: B.

Frage 2 (SK-SYM): Should the canvas show a switch for hiding constraint symbols?
A) No switch; automatic thinning only. B) A "Symbols" switch in the Sketches tab.
Recommendation A (no new control until large sketches show a need). Default: A.
