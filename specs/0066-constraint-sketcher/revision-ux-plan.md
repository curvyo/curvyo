# Sketch interaction revision

Status: Revision implementation authorized by the customer; revised product acceptance pending.
This document specifies the revision handoff and reviewable static study. Earlier artifact
approvals do not approve the revised implementation. The study is not solver evidence.

## Authority and unresolved details

Customer direction: preserve Curvyo's slim shell; make Sketch a recognizable object;
use continuous line/arc polyline modes and endpoint continuation, automatic constraints,
existing numeric-entry conventions, and predictable Escape/right-click cancellation.
Constraint creation belongs in the existing left rail and top options bar. Sketches on
the right inspects existing relationships/status; parameters get their own fx tab.
The explicit automatic-constraint request supersedes the earlier no-auto-constraint scope.
Detailed rules below are **(Proposal)** unless directly stated as customer direction.
Customer decisions, 2026-10-30:

- Tab option A is confirmed: enter numeric construction input and cycle relevant fields.
- Sketch has no snapping. Automatic axis constraints replace axis snapping; Vector keeps
  its existing snapping behavior. There is no held-Alt snapping inversion in Sketch.
- Alt is the preferred mode-switch key, consistent with Curvyo's other context-specific
  Alt operations. M is acceptable, but is not the primary binding in this revision.
- Each sketch has its own source page and is represented by one document object.
  Multiple-sketch creation and a sketch picker are unnecessary. Composition on another
  page uses existing Linked clones, not independent source-graph duplication.
- The sketch origin starts at the page centre.

These decisions supersede the previous interim M/Alt proposal and multiple-sketch UI.
Remaining detailed choices below are explicitly proposals, not unanswered key questions.

## Visual structure

Build the workbench choice from existing styled menu primitives and tokens, height, focus treatment,
chevron and spacing instead of an unstyled full-width native selector. Keep its existing
position above Properties and compact collapsed form; do not expand the pane on switching.
Use existing rail-card, icon, field and property-section components. No new font, palette,
card language, header, second options bar or bottom demo/status surface.

```text
  rulers / unchanged canvas                    [Vector / Sketch v]
  [Select]  [active tool | mode | relevant values] [vertical tabs]
  [Draw… ]                                      Sketches
  [Rules…]                                      Sketch / rename
                                                Status / inspect
       geometry + current-step preview          Existing constraints
                                                Selected geometry
                                                fx → Parameters
```

Each page owns one Sketch object with a human-readable name; internal IDs never appear
in labels. Remove New sketch, the active-sketch picker and numbered-sketch creation flow.
Selecting Sketch workbench directly edits the active page's sketch. Changing page selects
that page's sketch, never the previous page's graph. The object remains selectable in the
existing document/object list; Vector treats it as one object and Sketch edits its entities.
Return to Vector preserves the object and its completed edits. Composition pages contain
Linked clones of the source-page Sketch; entering editing through such a clone opens the
source page and original. Keep Linked clone discoverable through existing object actions.

**Question 7 — SKETCH-SEITE:** entering Sketch from a populated Vector page. A (recommended
safe proposal): create and switch to a dedicated Sketch source page. B: permit mixed
Vector content and a Sketch on that page. No response received during independent work;
proceed with A as a reversible **(Proposal)**, not customer approval of the detailed entry
flow. Never silently move, delete or convert existing Vector content. Navigation of an existing source page remains
history-free; explicit creation of a new page/object is a document operation.

**(Proposal)** entering the workbench alone does not write document history: the page's
empty sketch is available immediately, and its first durable graph edit materializes the
single object atomically. This includes defining a parameter before drawing geometry; the
fx workflow must not depend on a hidden New action or require drawing a disposable shape. A cancelled first gesture leaves no empty object behind. Undo,
delete/recreate, copy/paste, cloning and concurrent creation must respect one owning sketch
per page without silently merging graphs, discarding constraints or producing a second
editable page sketch. Review their precise document rules before implementation.

The local sketch origin is (0, 0) at the page centre when created: page coordinates
(width / 2, height / 2), not the viewport centre. Fixed-origin constraints refer to that
local zero. **(Proposal)** this is the initial placement; existing geometry is not silently
repositioned when a page is resized. Confirm the resize/placement rule in the next demo.
Keep the established page rulers/status coordinates; local Sketch input and dimension
labels use the sketch coordinate frame, with the origin visibly marking the distinction.

The existing left rail contains drawing and constraint tool groups, with coherent icons,
accessible names, tooltips and pressed states. Reuse the app's grouped-tool disclosure for
families; no permanent wall of disabled constraint buttons in the right pane. The single
top bar changes in place to current tool/mode and only relevant numeric settings. Existing
constraints remain compact inspectable rows; parameters and formulas live only in fx.

## Selection contract (Proposal)

| Gesture/context | Result |
|---|---|
| Select click on a point/endpoint | Select that point; do not also select its parent edge. |
| Select click on an edge | Select the analytic line/arc/circle/ellipse; no hidden helper points. |
| Click inside an unfilled closed profile | Empty-space action; avoid selecting distant edges by enclosure. |
| Overlapping hits | Nearest screen-space eligible hit; point before edge only within point tolerance. Repeated stationary clicks cycle tied hits with visible preselection. |
| Shift-click | Toggle target membership; selection intent does not move geometry. |
| Empty click / marquee | Empty click clears; marquee follows existing Curvyo containment/crossing conventions, Shift adds, Ctrl removes. |
| Drag a selected target in Select | Move only validated movable geometry; preserve constraints, show last valid result on rejection. |
| Constraint tool click | Collect eligible geometry for that relation, not transform it; show picked references and next eligible hover. |
| Annotation or constraint row | Select relationship and highlight its participants. Enter/double-click edits dimension or inspects geometric relation. |

Preselected eligible entities seed a constraint tool. Invalid members stay visibly selected
and cannot silently be ignored; top bar names required geometry and offers clear reset.
Do not rely on color alone for selected references or invalid input. Constraint tools use
the same point/edge pick rules with domain-provided eligibility, independent of draw order.

## Creation and cancellation (Proposal)

Polyline starts with one click; each subsequent click commits one valid line or arc and
starts its next preview without leaving the tool. A visible mode choice in the existing
top bar offers Line, Arc, Tangent, Perpendicular and Angle, with applicable modes enabled
only when a previous direction/curve exists. Show the geometric outcome before commit.
The verified FreeCAD comparison supplies a more precise cycle: perpendicular line,
tangent line, tangent arc, perpendicular arc in either direction, and free line.
Use these explicit line/arc choices rather than an ambiguous Tangent label alone; Angle
is an explicit numeric direction setting. Final naming/order is a reviewable proposal.

Starting on an eligible open line/arc endpoint resumes from that endpoint, with visible
endpoint highlight; it does not create an almost-coincident duplicate. This applies to
Polyline and the standalone start-first Arc tool, as the customer's original request states.
Arc Three points reuses the clicked endpoint as Start before collecting Through and End;
Centre-first construction retains its centre role and exposes a clear start-first resume route.
No tangent relation is inferred merely from selecting an endpoint. Resume also has a
discoverable selected-endpoint action in the existing options bar. Branching at a junction
must be an explicit start, not a guessed continuation. Closing on the start previews closure.

Sketch does not call the Vector snapping engine or use its enabled flag, X/Y masks,
targets or Alt override. Automatic inference is limited to the sketch axes: preview the
axis relationship before commit and persist a solver-valid constraint on confirmation.
**(Proposal)** an anchor on the local X axis receives y = 0, on the Y axis x = 0, and at
their intersection both. Use an explicit screen-space tolerance; show the proposed mark
and affected axis, and never add hidden arbitrary object/grid/angle constraints.

Chain connectivity, closure and continuation at an explicitly clicked endpoint are
structural relationships, not snapping. Reuse the existing endpoint identity where valid.
Tangent/perpendicular relations requested through a polyline mode are explicit construction
intent, rather than additional automatic target inference. The resulting relations remain
inspectable/removable. The mode/inference table must distinguish these three cases:
axis inference, structural endpoint connectivity and explicitly chosen mode relations.

**(Proposal)** a press of Alt cycles the active drawing tool's applicable modes once,
without committing a point or losing the pending chain; key repeat does not cycle again.
The existing top-bar mode control offers the same choices. Shortcut dispatch is scoped:
Alt during selection retains the established selection behavior, and text editing,
AltGr and system chords retain their existing handling. M may be a Sketch-only drawing
alias if it has no conflicting action; it is not required to discover or use the modes.

Escape and right-click cancel the current unfinished step; neither creates geometry.
For a polyline, discard the live segment and end continuation, retaining already committed
segments. With only an uncommitted first anchor, discard it. A second Escape returns to
Select. Right-click during a gesture consumes the context menu; idle right-click retains
the normal context menu. Press, release and contextmenu events must not execute left-click
creation or double-cancel. Switching tool/workbench cancels only unfinished state.

## Numeric and constraint workflows (Proposal)

### Finite mode and inference handoff

| Polyline mode, in cycle order | Required context | Preview and committed intent |
|---|---|---|
| Free line | Start point | Straight segment to pointer or typed length/angle; no direction relation. |
| Perpendicular line | Previous nonzero segment with endpoint direction | Line perpendicular to previous endpoint direction, explicit perpendicular relation. |
| Tangent line | Previous nonzero line/arc with endpoint tangent | Straight continuation of endpoint tangent, explicit tangent/collinear intent as supported by domain. |
| Tangent arc | Previous nonzero line/arc with endpoint tangent | Circular arc starting tangent to preceding entity, ending at next point. |
| Perpendicular arc left | Previous nonzero line/arc with endpoint tangent | Arc whose start tangent is perpendicular to prior endpoint tangent, left branch. |
| Perpendicular arc right | Same | Same perpendicular intent, opposite branch. |

Angle is numeric direction, not a seventh mode. Inapplicable modes stay visible with an
eligibility reason in the top-bar picker and are skipped by Alt cycling. Initial mode is
Free line; after choosing a mode retain it while applicable, without implicitly committing.
Alt keydown advances once; repeat, Ctrl/Meta chords and AltGr do not advance. Pending
anchor/chain survives. Alt in Select retains existing marquee behavior. Arc construction
variants cycle within the active Arc tool; they do not switch tool families. Top-bar mode
selection provides the pointer/keyboard alternative. M is an optional drawing alias.

#### Standalone Arc variants (Proposal)

Use exactly two variants in the existing top-bar mode menu, in this Alt cycle order:

| Variant | Pick order | Preview and numeric Radius |
|---|---|---|
| Three points | Start → Through → End | Circular arc from Start to End through Through. After two picks, pointer movement previews the third point and resulting arc. Radius is derived, not an editable construction input; three pinned points cannot also accept an independent radius. |
| Centre, start, end | Centre → Start → End direction | Centre and Start define radius; pointer direction supplies End on that circle. Show the radial guide and projected endpoint before the final click. Radius can be entered after Centre and before Start; it fixes the preview radius while the pointer chooses Start direction. Once Start is picked, radius is derived/read-only so neither pinned point moves. |

Each left click pins one pick; intervening movement changes only the next preview. These
standalone Arc variants use click–move–click steps, not a drag-release commit. A typed radius
does not fabricate an angle or commit the arc: Enter accepts the radius and returns to
Start-direction picking. Final left click commits one valid arc. No extra toolbar or
radius field that appears editable while its geometry is already pinned.

On Alt or menu mode change, preserve every existing pick's exact coordinates and order.
Change their explicit on-canvas role labels: pick 1 Start ↔ Centre; pick 2 Through ↔ Start.
The top bar identifies the next role (End or End direction); redraw only provisional
geometry. This is an announced reinterpretation of unfinished picks, never a transformation
of committed geometry or silent reset. Any typed Radius setting belongs only to Centre,
start, end; it cannot override preserved picks after switching. If two picks exist, their
distance takes precedence and Radius is derived. Alt inside numeric text retains its
existing text/modifier handling rather than changing modes.

Reject coincident/collinear Three points, zero-radius Centre/Start, or an invalid final
direction without committing or replacing the first two picks. Keep the last valid preview
where available, identify the invalid next role and allow pointer/numeric correction.
Escape from numeric entry discards that draft first; Escape on canvas or right-click
cancels the unfinished arc and all its pending picks, preserving previously committed
geometry. Subsequent Escape returns to Select. Right-click never supplies an arc point.

| Relationship source | Result |
|---|---|
| Pointer near local X axis | Preview y = 0; commit only valid axis constraint with point. |
| Pointer near local Y axis | Preview x = 0; commit only valid axis constraint with point. |
| Pointer near local origin | Preview/commit both coordinate constraints when valid. |
| Explicit existing endpoint pick, closure or resume | Reuse endpoint identity/connectivity; structural, not inferred snapping. |
| Chosen tangent/perpendicular mode | Explicit requested relation, not automatic inference. |
| Other geometry, grids, arbitrary angles, Vector snap flags | No automatic inference and no snapping in Sketch. |

Numeric order: line length → angle; centre/radius circle or arc radius; rectangle width →
height; ellipse first axis → second axis; slot width; point X → Y. Diameter is an explicit
alternative to radius, never an extra required field. Tab enters first relevant field;
Tab/Shift+Tab cycle applicable fields while entry is active. Enter commits one valid step.
Escape in a field discards that numeric draft and restores canvas preview/focus; next Escape
cancels the unfinished step. Right-click on canvas cancels the whole unfinished step and
draft immediately. Fields retain native text undo. Fixed earlier anchors never follow a
later typed edit. Three-point modes collect required points before a dimension can apply.

### Construction entry and dimensions

Tab from an active canvas construction enters its existing numeric-entry flow, with the
current relevant value selected; subsequent Tab/Shift+Tab follow visible field order.
Line: length, angle. Circle/arc: radius or diameter where applicable. Rectangle: width,
height. Reuse existing inline entry/placement styling; do not create a second options bar.
This Tab behavior is confirmed by the customer (option A), not an open question.
Enter accepts one valid step, returning to continuous drawing where applicable. Invalid
input remains focused with an associated error; native text undo and modifier tracking
remain intact. Escape in numeric entry first discards its draft and restores canvas
preview/focus; the next Escape cancels the unfinished construction. Right-click on canvas
cancels the unfinished construction including its numeric draft.

Dimension tool: a line preselection offers length; two points offer distance; two lines
offer angle. One circular edge offers radius/diameter via an explicit top-bar choice.
Do not guess centre-to-edge versus endpoint distance. With no selection the tool collects
the required geometry. Preview annotation placement, then numeric entry; commit creates
one dimension. An existing dimension opens its expression directly, without duplicating
the relation. Reference mode always shows measured read-only output; switching to driving
is explicit. Radius editing must be discoverable from circular geometry selection, not
require locating an unrelated properties button. Other relations use the same pick-preview-
commit pattern, completing immediately only when all eligible references are present.

fx contains named parameters, units, expression and resolved value. Inline edits use the
same entry component and native keyboard conventions; invalid expressions preserve the
last valid geometry. Sketches keeps constraint conflict inspection/repair and DOF status,
with links to affected geometry and parameter entries. No diagnostics migrate to a footer.

## Staged review before implementation

Static stage is available in `demo/index.html`; production implementation is authorized
separately. This file uses token-matched standalone menu markup to represent the existing
app component, not a new production menu implementation. All scripted-state disclosures
and scenario controls stay inside the Properties pane.

Capture sequence for the lead's exclusive browser session:

- Initial view: connected line/arc/line with one preview, slim Sketch inspector and styled
  workbench/menu; no picker, New button, snapping control or bottom demo bar.
- Click **Line → arc → line** inside Interaction study, then canvas twice: line with arc
  preview → tangent arc placed with line preview → final line placed. Capture last state.
- With canvas focused, Alt advances mode once; Tab focuses Length, next Tab Angle;
  Escape discards numeric draft, next Escape ends preview, preserving completed geometry.
  Replay and right-click canvas to exercise the same cancellation without point creation.
- Click **Radius dimension**, then canvas: Radius input receives focus. Enter a positive
  value and press Enter; measured picture is illustrative, not recomputed by a solver.
- Click **Axis constraint** for the local-axis preview. Click fx for separate parameters.
  Open the workbench menu or collapse Properties to capture compact control variants.

Only the named storyboard paths are scripted. Other rail tools illustrate their placement
and selection, not complete working construction. Root must review/capture the static
artifact; no browser or executable check was run by this docs/demo author.

Subsequent authorized static-demo check: after the lead handed over exclusive browser
ownership, `target/0066-browser/ux-demo-check.cjs` passed in Chromium at 1440×1000 and
1000×650. It exercised chain steps, Alt, Tab/Shift+Tab, numeric Escape, construction Escape,
right-click cancellation, resume, radius entry, menu keyboard focus, collapsed workbench,
fx switching and absence of horizontal overflow/page errors. Fixed the static SVG hidden
state handling, kept Page 1 on one line and replaced Add parameter's glyph with SVG.
Reviewed refreshed `revision-demo-sketch.png` and `revision-demo-compact.png`; fx screenshot
also refreshed. Browser closed and ownership returned to the lead. This is a static-study
check only, not production UX approval, solver verification or customer acceptance.

1. Review shell composition as a static study: styled workbench choice, recognizable sketch
   object, grouped left tools, contextual top bar, lean Sketches and separate fx. Compare
   directly with existing Vector components, including collapsed panel and narrow height.
2. Review a scripted interaction storyboard: active page → Sketch → line → tangent arc → line;
   preview/cancel each step; resume an open endpoint; Tab numeric entry and invalid input.
   It must explicitly identify scripted geometry, not claim a working solver.
3. Review selection and constraints: overlapping endpoint/edge, multi-selection, radius
   from circle selection, line length, reference measurement, axis-constraint preview,
   conflicting relation rejected/repaired, parameter formula propagation.
4. Use the finite Alt-mode/axis-inference handoff above with PO/architecture. Reconcile the
   spec and ADRs with one owning sketch per page and page-centred origin; include page
   switching, first commit/undo, resize, deletion, cloning and concurrent creation cases.
   Tab/Alt decisions are settled and implementation is authorized. Review remaining
   proposals in the demo and verify the implementation with independent acceptance cases.
5. Implementation review later must exercise keyboard-only access, focus after cancel,
   right-click without accidental creation, one sketch per page after save/reopen, centred
   origin on differently sized pages, undo per committed segment,
   reference-value correctness and selection stability. No test or performance PASS here.

## Design evidence

Used local `.claude/skills/ui-ux-pro-max/SKILL.md` and focused UX searches for keyboard
focus/escape and progressive disclosure. Relevant guidance is visible, unobscured focus,
escape routes and progressive contextual controls. Existing `docs/design-system.md`
takes precedence over generic mobile sizing, fonts or new visual styles. Official FreeCAD
comparison from the lead's first-party documentation review:

- [Create polyline](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/Sketcher_CreatePolyline.md):
  successive line/arc segments, existing-endpoint starts, M mode cycling, applicable auto
  constraints and Escape/right-click finishing an open chain.
- [Dimension](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/Sketcher_Dimension.md):
  selection-context suggestions, additional picks updating the suggestion, M type cycling,
  canvas placement followed by value entry, Escape/right-click ending the tool. Adopt this
  as the context-dimension proposal above, with visible alternatives in the existing bar.

Actual production observations, reported by the lead's sole-browser audit:

- `target/0066-browser/revision-current.png`: native full-width workbench and Entities
  selects, random sketch suffix, and fifteen mostly disabled constraint actions consuming
  the Properties area. These substantiate the requested composition/component revision.
- `revision-audit.json`: arc left-click start → right-click second point → left-click third
  point creates an arc. Right-button creation is a confirmed defect, not a style preference.
- Tab after the first polyline pick focuses the Workbench select instead of numeric entry:
  confirmed inconsistency with the requested construction workflow.
- Four polyline clicks produced three lines and their points; Escape retained that chain.
  The reported single-segment limitation was not reproduced by this basic sequence. Keep
  the customer's actual problematic sequence open as a regression priority; do not infer
  that every mode/resume/cancel flow works from this one successful path.
- In the focused-canvas sequence, Escape discarded a pending polyline start: the next
  click produced no geometry, and the following click created exactly one new line.
  This confirms that one cancellation path works; the customer's failed Escape path remains
  open, including numeric focus, drag, arc phases and repeat cancellation. Do not mark the
  reported failure resolved or substitute this baseline for those additional cases.
- `revision-interaction.png` accompanies the interaction audit. Green CI does not override
  the customer's UX rejection.

This reviewer ran only local skill searches and wrote this plan. Production browser
observations and first-party research above are attributed to the lead; no implementation,
build, browser operation or independent interaction PASS is claimed here.
