# Sketch UI interaction map

Status: Accepted 0066 interaction map implemented; source/artifact UX review APPROVE and independent browser acceptance passed. See ux-notes.md and acceptance-session-report.md.
Authority: `specification.md` AC1–18 and `ux-notes.md`. The customer approved the existing
Curvyo shell, not a new frame. Reversible details below are **(Proposal)**. The static demo
illustrates layout only: its scripted states, hard-coded parameter ranges and illustrative
construction choices are not independent product requirements.

Navigation evidence: ProjectAtlas exact summary for `PropertiesPanel.tsx` returned
`refresh_required`; the already timed-out refresh was not restarted. Bounded reads of that
file, `ToolRail.tsx` and `ShapeToolbar.tsx` informed the existing-shell integration below.

## Placement and ownership

| Existing surface | Sketch behaviour | Preserve |
| --- | --- | --- |
| Properties container | Workbench selector above body/vertical strip; Vector / Sketch | Existing collapse preference and camera-origin compensation |
| Collapsed 32px strip | Compact V/S native choice, accessible name Workbench; full option names | No new header or forced expansion; coarse-pointer width follows existing strip |
| Properties vertical tabs | Sketches contains picker, status, constraints, dimensions, parameters | Existing roving tab focus, expanded automatic / collapsed manual activation |
| Left floating tool cards | Sketch-specific Select, Point, Polyline, Arc, Circle, Ellipse, Rectangle, Slot, Fillet | Existing card placement, icon targets, keyboard navigation and pointer focus return |
| Single floating top options bar | Active construction variant and relevant inputs only | Existing overflow second-row pattern; no second constraint bar or viewport shift |
| Canvas | Sketch-local geometry, origin, annotations and selection/diagnostic highlighting | Rulers, pan/zoom and geometric feedback; no diagnostic banner |

All DOF counts, solver state, error explanations and repair controls live in Sketches.
No app header, breadcrumb, Finish button, bottom state/demo bar or app-status diagnostics.
Do not bring ordinary Bézier Pen, Star/Polygon, Boolean/path actions into Sketch implicitly.
Expose solver-supported operations from domain state; the UI never infers validity merely
from the number of selected nodes or silently edits unrelated document objects.

## Navigation and shared input rules

1. Vector → Sketch restores the active sketch; when none exists, Sketches exposes New sketch
   and the picker without creating a hidden sketch by navigation alone. New sketch creates
   one editable object in the current valid layer/context and one named history step.
2. The picker in Sketches names the current sketch. Choosing another cancels only the
   unfinished gesture/entry, retains completed edits and changes editing scope. Vector
   returns to document editing with the sketch object selected; neither switch writes history.
3. Workbench switching retains collapsed/expanded state. Explicitly choosing Sketches opens
   the panel via its existing rule. Keyboard selection retains control focus; pointer actions
   return focus to canvas. Sketches remains active through geometry selection/tool changes.
4. No new global tool-letter assignments. Tab/Shift+Tab reach rail/options/panel using existing
   order; Space/Enter operate focused buttons. Native select arrows choose named variants.
   Tooltips name actions, variants and unavailable reasons, not permanent operating hints.
5. Escape restores the current numeric entry or cancels an unfinished construction/drag;
   a subsequent Escape clears selection. It never discards completed sketch edits or exits
   the workbench. Return focus to canvas after cancelling/committing an inline entry.
6. Delete on a focused constraint row removes that relationship, not its geometry. Delete
   in text fields remains text editing; canvas entity deletion must identify affected
   references and use the agreed domain operation, never leave dangling constraints.
7. Outside typing, Ctrl/Cmd+Z and existing redo chords use document history. Inputs retain
   native/local text undo. Preview and inspection are not history steps; completed edits are.
8. Switching construction variants cancels the unfinished picks only; committed entities
   remain. Show the current stage by construction-point/geometry preview; no new hint banner.

## Tool action matrix

Each numeric entry uses units and the existing inline-entry styling. Enter accepts a complete
valid construction/entry; incomplete constructions keep their current picks without creating
geometry. Escape follows the shared cancellation rule. No solver failure commits a fabricated
result. Keyboard equivalents use named options and inputs, not undiscoverable modifiers.

| Tool / variant | Selection / pick sequence | Preview, completion and cancellation |
| --- | --- | --- |
| Select / Entities | Hit point, segment or analytic curve; Shift extends selection | Drag free geometry with hard constraints; release one step, Escape restores. Sketches shows applicable actions and fixing relationships. |
| Select / Constraints | Pick annotation or Sketches row | Highlight its referenced entities; no geometry move. Enter edits a numeric dimension or invokes Inspect for geometric relations. |
| Point | One position; optional coordinate entry | Preview prospective point; click or complete coordinate entry commits one point. |
| Polyline / Connected lines | First point, then successive endpoints | Each completed segment remains; shared endpoints are structural. Enter finishes open; click first point closes; Escape drops only the pending segment. One named operation per completed segment; ending the chain adds none. |
| Arc / Centre and endpoints | Centre, start, end | Preview radius and sweep; final point commits. Keep degenerate picks editable, with refusal in Sketches. |
| Arc / Three points | Start, point on arc, end | Preview analytic arc; collinear/duplicate picks refuse completion and retain earlier picks. |
| Circle / Centre and radius | Centre, perimeter point or radius entry | Preview centre/radius; final click or valid radius Enter commits one circle. |
| Circle / Three points | Three distinct circumference points | Preview analytic circle at final pick; collinear input identifies the invalid construction without deleting prior picks. |
| Ellipse / Centre and axes | Centre, first-axis endpoint, second-axis extent | Preview analytic axes/perimeter; final point or complete axis entry commits. Reject zero/invalid axes without replacing last valid geometry. |
| Ellipse / Axis endpoints and point | Two ends of first axis, point determining second extent | Preview the same analytic ellipse representation; last pick commits. |
| Rectangle / Opposite corners | Two corners | Preview connected line geometry; final click/complete size entry commits the rectangle with its intended rectangular relationships. |
| Rectangle / Centre and corner | Centre, corner | Same rectangle model, different anchor construction; dimensions describe full width/height. |
| Slot / Centreline and width | Two spine endpoints, width/perimeter pick or width entry | Preview parallel sides and tangent semicircular caps. Complete construction is one step, with intended relationships retained. |
| Fillet / Radius | One valid shared corner, or two adjoining entities; radius input | Preview trimming/tangency and affected references. Enter commits one operation only if constraints can be retained/remapped; Escape restores originals. Failure names affected constraints in Sketches. |

Centre-based Rectangle/Arc variants above are optional interaction proposals, not extra
accepted criteria or required constructor families. An implementation may derive the same
accepted geometry through the existing domain creation API. Canonical supported entity/
constraint combinations follow `core-api.md`; this map does not extend that contract.

The demo's Polyline “Line and arc” choice is not part of the accepted straight-polyline
operation; do not implement it from mock text. No curved slots or arbitrary new construction
families follow from the selector pattern. Snapping aids eligible picks but never adds
unrequested persistent relations. Construction geometry, if provided by the agreed model,
uses an explicit labelled control; it is not an automatic consequence of selection.

## Constraint action matrix

Every action is discoverable in Sketches. Invalid actions remain focusable with
`aria-disabled`; activating them changes nothing. The tooltip/reason names the required
selection. Eligible pairs must also pass domain geometry checks (same active sketch,
valid references, appropriate entity kinds). Unordered pairs must not depend on click order.

| Action | Required selection / meaning | Invalid selection reason example |
| --- | --- | --- |
| Coincident | Two distinct sketch points, including explicit endpoints/centres | Select two points. |
| Horizontal | One straight segment, or two points; equal y coordinates | Select a line or two points. |
| Vertical | One straight segment, or two points; equal x coordinates | Select a line or two points. |
| Parallel | Two straight segments | Select two lines. |
| Perpendicular | Two straight segments | Select two lines. |
| Tangent | Two eligible analytic entities with a supported tangent relation; shared endpoint when endpoint tangency is required | Select a supported line/curve pair. / Select adjoining entities. |
| Equal | Two lines for equal length, or two circles/arcs for equal radius; no mixed length/radius interpretation | Select two lengths or two radii. |
| Fixed | One point or one supported entity; freezes its current defining geometry | Select geometry to fix. |
| Symmetric | Two points and a selected straight symmetry axis | Select two points and a symmetry line. |
| Midpoint | One point and one straight segment; point coincides with segment midpoint | Select a point and a line. |

Entity-kind combinations must match the solver contract; unsupported combinations remain
explicitly unavailable rather than approximated. For example, Equal must not guess whether
two ellipses mean equal major axes, minor axes or both. Additional supported semantics need
an explicit label before use, not an invisible reinterpretation of the same action.
Valid activation adds one relationship/history step and retains entity selection for further
constraints. Redundancy/conflict is inspectable; it does not trigger automatic deletion.
The origin is an explicit selectable item with a Fixed state: use the domain-supported
Fixed toggle/release/move operations. The core may store this as an entity flag rather than
a constraint row; do not synthesize a removable relationship or treat a fixed origin as free.

## Dimension and parameter editing

| Action | Selection / entry | Commit and focus |
| --- | --- | --- |
| Length | One straight segment | Inline value/units; Enter writes one driving dimension. |
| Distance | Two explicit points; named distance mode if horizontal/vertical alternatives exist | Do not silently choose centre/edge references. Show the referenced points while editing. |
| Radius / Diameter | One circle or circular arc | Field names radius versus diameter; switching entry kind must preserve represented size. |
| Angle | Two straight segments or a supported arc-angle reference | Preview selected angle branch; no silent supplementary-angle change. |
| Reference measurement | Same eligible references, explicitly Reference mode | Read-only resolved value, visually labelled Reference; not a disabled driving-value field. |
| Edit dimension | Double-click annotation, or focus row and activate Edit/Enter | Same inline editor/value model. Enter commits once and returns canvas focus; Escape restores; Tab commits valid entry and advances. Invalid input retains focus with adjacent error in Sketches. |
| Add parameter | Add parameter button in Sketches | Inline Name, Expression/value and Unit row, not a dialog. Enter commits complete valid row; Escape drops draft and returns to Add. |
| Edit parameter | Activate name/expression/unit field | Show resolved value separately from expression. One commit updates dependents; no intermediate character becomes a document step. |
| Inspect dependency | Select parameter/dimension row, Inspect | Highlight dependent dimension rows/annotations; no automatic geometry edit or navigation to another workbench. |

Names must be nonempty and valid for the expression grammar; duplicates, unknown names,
cycles, invalid expressions and incompatible units identify the field/dependency to repair.
Preserve the draft for correction and retain last-valid resolved values/geometry. Rename
must use reference-aware domain behaviour; never silently orphan formulas. Do not copy the
mock's 70–100 range or readonly sample rows into production. Re-read committed/normalized
values after edits and undo/redo. Reference values update from the solved result.

## Diagnosis, inspection and repair matrix

| State / action | Sketches presentation | Explicit user route |
| --- | --- | --- |
| Under-constrained | State text + valid DOF count | Show selects/reveals free entities and permitted motion; entity Inspect links fixing/free relationships. |
| Fully constrained | State text + 0 DOF | Inspect remains available; select geometry to see the constraints fixing it. |
| Redundant | Distinct state and implicated rows | Select group/row highlights geometry; Edit or Remove chosen relationship. Do not label redundancy contradiction. |
| Conflicting | State text, implicated constraints, last-valid outline | Select row/group; Edit dimension/parameter or Remove constraint; one repair step, undo restores the prior state. |
| Solver failure | Failure state, count unavailable unless valid evidence exists | Preserve geometry and proposed edit context; permit explicit edit/revert/undo. Do not invent a conflict set. |
| Remove constraint | Focused selected relationship, Delete or named Remove button | Remove only that relationship; keep surviving row focused, refresh diagnosis/geometry. |
| Convert to paths | Active sketch, explicit command in Sketches | One undoable domain conversion; no constraint symbols/dimensions exported. Return to Vector with resulting paths selected because no editable sketch remains. |

Use text state plus canvas/row emphasis, not colour alone. Announce committed state changes
once through the established accessible status mechanism; never announce DOF every drag
frame. Visible diagnostics remain exclusively inside Sketches, including refusal details.
Inspection pans only when needed to reveal the selected geometry and never changes history.
Show permitted motion only from validated solver information. A DOF count alone does not
justify movement arrows or per-entity freedom claims; never invent vectors to fill the UI.

## Implementation order within the existing UI

1. Wire workbench/active-sketch state into existing rail, one options-bar owner and Properties;
   preserve existing tab activation/collapse handling and keep state authority in the session.
2. Add Sketches picker/status/selection plus command availability; connect named domain actions
   to tools/constraints before adding any mock-only state. Do not expose nonworking controls.
3. Reuse existing entry/focus patterns for dimensions and parameters, backed by one domain
   command per commit; synchronize panel and canvas after commit/undo/redo.
4. Connect selection/inspection and real solver diagnostics, then verify all matrix routes
   with keyboard and pointer, collapsed Properties, cancellation and failed edits.

This document supplies interaction details, not implementation/test evidence. No production
code, build, browser, solver test or new scope is authorized by the mock itself.

## Pre-wiring integration review after the Trace Mask rebase

Source/doc review only; no frontend exists yet and no UI PASS is implied. `session-plan.md`
§6 matches the accepted shell and command ownership. Preserve these concrete integration
requirements while applying it:

- `App.tsx` currently chooses Vector bars by `editor.tool`, including Select and the Trace
  bar, inside one `data-bar-area`. Branch by workbench before tool name so Sketch Select
  cannot also render the Vector Select bar. Reuse that single owner, viewport anchoring,
  rail-clearance offset and history-preview inert state; do not add another overlay row.
- Preserve the rebased Trace bar/View pill's Overlay, Mask, Trace and Image controls and
  callbacks in Vector. `useTraceViewMode` installs global peek/cycle listeners while active;
  entering Sketch must end Trace peek/mask context and deactivate these listeners, not
  merely hide the Trace bar. Returning Vector must not leave an invisible mask overlay.
- `PropertiesPanel` has a contextual About Trace tab and separate `aboutActive` header/body
  routing. Entering Sketch must route to Sketches rather than leave that contextual pane
  selected. Add Sketches through the domain tab registry and the existing glyph/ARIA map;
  do not create an independently focused second tablist for its content sections.
- Place the Workbench selector outside the conditionally mounted Properties body so it
  survives collapse. Native-select changes must not call the generic `activate` handler,
  which unconditionally expands Properties. Compact V/S retains full accessible option
  names; tab activation, not workbench switching, uses the existing expansion rule.
- Preserve `CanvasArea`/panel camera-origin compensation and current font-preview layout;
  do not duplicate a width correction when the selector is added. Avoid a full-canvas
  header and keep rulers/left tool cards anchored to the existing viewport.
- Existing panel focus return prioritizes the text-edit sink. End Vector text editing via
  the proper domain transition before Sketch focus return; do not send sketch input back
  to a stale hidden text field. Keep native undo in actual entry fields and allow modifier
  events through to the central tracker; no broad `stopPropagation` for every field key.
- Annotation overlays need pointer events only on real interactive labels. Their activation
  must not bubble into entity creation or begin a canvas drag; pointer-up/capture cleanup
  remains paired. Keep noninteractive annotations transparent to canvas picking. The
  equivalent row Edit/Enter route must open the same dimension draft, not a second editor.
- Constraint availability comes from Rust. Use focusable `aria-disabled` plus a dispatch
  guard, not native disabled, for invalid actions. Select options, parameter labels,
  expression errors and readonly resolved values need explicit accessible names/relations.
  After deleting a relationship or switching drafts, retain a surviving focus target.
- Keep independent selected-row, affected/conflict-row and keyboard-focus styles. Only use
  solver-provided entity/constraint IDs and validated freedom information. All diagnostic
  text, refusals, counts and repair entries remain inside Sketches, never CanvasNotice or
  an added bottom strip; any screen-reader status announcement stays visually hidden.

This list identifies producer risks, not observed defects in an implemented frontend.

## Initial frontend source review (implementation in progress)

Bounded read-only review of `components/sketch`, `useSketch`, `sketchView` and existing-shell
wiring identified the following concrete corrections, sent directly to the producer:

1. Reset/key dimension, new-parameter and numeric construction drafts by active sketch ID;
   an old draft must never submit into a newly selected sketch graph.
2. Restore a surviving focus target after cancelling parameter creation or deleting a
   parameter. Reopening the same dimension editor after Escape must refocus it even when
   the editing-ID/autoFocus boolean has not changed.
3. Associate field errors with their inputs, not only a shared alert. Reconcile successful
   normalized/no-op commits with authoritative displayed values even if the value prop did
   not change; invalid drafts remain editable.
4. Label solver-classified entities Fully constrained, not Fixed: the latter would imply an
   explicit fixed relation/flag that may not exist.
5. Ensure full constraint names fit the real panel width: three columns with horizontal
   padding can overflow for Perpendicular/Coincident. Use an arrangement that keeps names
   readable without widening the panel or truncating action meaning.
6. Numeric-toolbar Escape must cancel domain picks/fillet preview as well as host text.
   Plain pointer annotation inspection returns canvas focus; dimension Edit must focus its
   input, including repeated editing of the same relationship.

Follow-up bounded source inspection found active-sketch keys, nullable authoritative field
drafts, field error association, two-column constraint actions, corrected Fully constrained
copy, domain cancellation on numeric Escape, annotation pointer focus return, repeated
dimension refocus and parameter cancellation/deletion focus fallbacks in place. New
dimension/parameter and toolbar numeric draft inputs still need their command errors
associated with the input; cancelling new-parameter creation should discard its draft.
These source observations do not establish runtime or visual correctness.
Workbench-first bar routing, selector outside the collapsed body, stored-state reads and
Trace-context gating are present in source. Browser density/focus/interaction evidence is
still required. No browser, build, test or production edit was made by this reviewer.

Final source/artifact follow-up: the remaining new-draft error associations and parameter
Cancel discard are now present. Refreshed production `plate.png` shows native collapsed
Geometry and compact relationships resolving the observed density issue. Source/layout
review is approved within the evidence limits recorded in `ux-notes.md`; independent
interaction acceptance remains the tester's responsibility and is pending at this review.
