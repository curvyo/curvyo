# 0060 Snapping: UX notes

Status: UX APPROVE for the explicit creation-Alt extension; full gate and customer acceptance remain separate.
The current extension's verdict is in §11.4; §9 and §10.6 retain earlier review history.
Owner: ux-engineer. Authority: customer requests of 2026-10-17, 2026-10-29
(document-owned settings and six page fractions), and the latest request relayed by the
lead on 2026-10-30: a dedicated Snapping entry in the Properties rail, ordinary click opens
its panel, Shift-click toggles document snapping without changing the visible panel or its
collapsed state, and enabled snapping has a subtle blue indication. No narrow bar above
the panel. This latest instruction supersedes the former Select-bar toggle / Document-block
placement in criteria 2 and 10 and question SNAP-HOME; the product owner updates that spec.

References: `docs/design-system.md`, especially Properties panel: tabs, Switch, Reference
mark and keyboard conventions. **(Proposal)** marks reversible implementation choices.
Defaults stay off, all four kinds on, capture 8 px, release 14 px; release is never smaller
than capture. The latest PR #225 feedback adds independent X/Y controls, temporary Alt
inversion during an established move, and drawing hover/first-click snapping (§10).
The customer has now explicitly extended Alt inversion to every Pen/Rectangle/Ellipse/
Polygon-Star creation phase (§11), including Ctrl+Alt. Unrelated tools keep their existing
modifier rules. Settings remain document-owned and undoable;
active pane, collapse state, held targets and markers remain view/interaction state.

## 1. Snapping rail entry and settings panel

### 1.1 Findings in the inherited implementation

Read-only source review of `PropertiesPanel.tsx`, `PanelStrip.tsx`, `SnappingToolbar.tsx`
and `useSnapping.ts`, 2026-10-30:

1. `PropertiesPanel` renders a separate 24px `SnappingToolbar` above each non-Trace header,
   contradicting the latest placement instruction; collapsed panels expose no snapping control.
2. That toolbar has only an enabled toggle and passive target names. There is no target or
   zone editing UI, nor a Snapping rail entry or sticky Snapping pane.
3. Existing rail activation is automatic on focus while expanded. Intercepting Shift-click
   only in `onClick` is insufficient if the tab primitive already activates on pointer-down
   or focus: the modified gesture must be kept out of the activation path entirely.
4. The inherited notes specify whole numbers 1–100, but the criterion only rejects negative
   and non-finite values. Remove that invented restriction; finite nonnegative values are
   valid, including 0 and fractions. Preserve existing capture/release normalization.

These are implementation findings, not final acceptance. No browser or build was run by
this reviewer. The marker and snapping engine contracts below otherwise remain in force.

### 1.2 Entry, activation and toggling

- Add one persistent **Snapping** tab to the existing vertical Properties strip. **(Proposal)**
  append it after the persistent Fonts tab and before contextual About Trace, keeping all
  existing permanent entries in their order. Use Lucide `Magnet`, 16px, 1.5px absolute stroke,
  the existing 28px hit target (36px coarse), spacing, scroll behaviour and inset focus ring.
- A plain click opens the body and selects Snapping. It never changes `snapping_enabled`.
  Plain Enter/Space does the same. Arrow/Home/End navigation follows the existing strip:
  expanded activates on navigation, collapsed only moves focus. No extra top toolbar,
  Select-bar toggle or duplicate Document settings block remains.
- Shift-click toggles `snapping_enabled` once and preserves the currently displayed pane,
  its scroll position, and whether the body is expanded or collapsed. It works when another
  tab is selected and when Snapping itself is selected. It must not briefly select or expand
  Snapping before restoring the previous pane. The rail position must not jump.
- **(Proposal)** Shift+Enter and Shift+Space on the focused Snapping tab are the keyboard
  equivalent, once per physical press, without changing the active pane/collapse state.
  Prevent the additional synthetic click and Space scroll; ignore repeats. This is local
  to that tab, not a canvas/global shortcut. A plain Space/Enter still opens the panel.
  Intercept modified pointer/key activation before the tab primitive's activation/focus
  handling. Modified activation must not change the roving active-tab state indirectly.
- Pointer actions return focus to the canvas according to the strip's convention. Keyboard
  actions retain focus on the tab. Escape returns to the canvas without clearing selection.
  No snapping action steals focus from a text field except the user's deliberate navigation.
- Keep the panel sticky through selection changes, deselection, group entry and tool changes,
  like History/Layers/Fonts. Only deliberate navigation changes it. Hide/show retains it;
  New/Open keep the existing panel reset policy, and load the new document's snapping state.
  Canvas-drag gating remains as for other settings: no mid-drag panel mutation. The
  move and creation Alt inversion is specified in §10–11; outside those scopes the
  existing Ctrl+Alt suspension route remains.

### 1.3 Independent enabled, selected and focused states

`aria-selected` and the usual strong selected-tab fill mean only that the Snapping pane is
open. They must never represent whether snapping is enabled. **(Proposal)** use the following
state table; no hard-coded blue or reserved shadcn `--accent` token:

| Pane shown | Snapping enabled | Appearance |
|---|---|---|
| No | No | Ordinary unselected rail entry |
| No | Yes | Subtle `--editor-accent-hover` ground, editor-accent magnet, small enabled badge |
| Yes | No | Standard strong selected-tab ground/foreground, no enabled badge |
| Yes | Yes | Standard strong selected-tab ground/foreground, enabled badge in the foreground colour |

**(Proposal)** the enabled badge is a small check mark inside the icon button, with enough
clearance to keep the magnet legible. It distinguishes state without relying on colour;
its casing follows the current ground as the File dirty badge does. The existing two-tone
inset focus ring stays visible independently of all four combinations. Hover must not erase
enabled or selected state. With the body collapsed, no tab is selected but the enabled
indication stays visible. Temporary suspension or move/creation Alt inversion never changes
the persisted enabled indication. Do not recolour the rail or toggle the panel switch to
represent effective transient snapping; the canvas marker follows the effective result.

Keep the tab's accessible name `Snapping`, its `role="tab"`, `aria-selected` and panel
relationship. An accessible description names `On` or `Off` separately; do not give a tab a
second incompatible switch role. **(Proposal)** tooltip: `Snapping — on` / `Snapping — off`,
with a short second line `Shift-click to toggle` (keyboard focus: `Shift+Enter or Shift+Space
to toggle`). This is shortcut discovery in the existing tooltip, not a persistent hint label.
A modified toggle can announce `Snapping on` / `Snapping off` once via the existing status
mechanism; never announce capture/suspension on every frame.

### 1.4 Panel controls and focus

Use the normal fixed header `Snapping`, 280px body / 244px content, normal body padding and
one vertical scroll area. No selection subject is required. The content is always available
when Snapping is active, including without selected objects or with snapping switched off.
No additional popup, banner, explanatory paragraph or second toolbar.

| Order | Control |
|---|---|
| 1 | Label `Snapping`, an explicit labelled on/off switch using the established Switch component; `role="switch"`, `aria-checked`, accessible name `Snapping`. This is the touch and unmodified-keyboard alternative to Shift-click. |
| 2 | `Snap targets`: Objects, Guides, Nodes, Page as four independent pressed-state toggles, in the established two-column layout. They stay editable with global snapping off and do not switch it on implicitly. |
| 3 | `Snap axes`: independent `Horizontal (X)` and `Vertical (Y)` pressed-state controls, both on by default; coordinate-component semantics and history in §10. |
| 4 | `Capture`: existing panel number-entry pattern, displayed in screen `px`, accepts finite nonnegative values. |
| 5 | `Release`: same; below Capture normalizes to Capture and shows the stored result after commit. Raising Capture beyond Release raises Release in the same document transaction. |

Tab order is controls in visual order (the header is not focusable). On keyboard tab opening,
focus stays in the strip; Tab enters the first switch. All controls use existing token colours,
field backgrounds, readable text, invalid styling and focus rings. Touch can perform every
operation without Shift or hover. No controls disappear when the main switch is off.

A field commits once on Enter, Tab or edited blur; unchanged text writes nothing. Enter commits
and returns focus to canvas; Tab commits and advances. Escape restores the stored value and
returns focus without writing. Rejected text leaves the document unchanged, marks the field
invalid and uses the existing validation chip, `Enter a non-negative number`. No arbitrary
upper bound or integer-only constraint is added. Show the normalized stored value after a
commit; Undo/Redo must refresh switches, fields, the rail badge and the dirty indicator in
the same UI update.

### 1.5 History and native text editing

All enabled, target and zone changes are document history steps; navigation, focus and
collapse are not. Shift activation and the panel switch use the same setting operation.
Changing Capture and its dependent Release creates one step. Toggling twice is two steps;
opening the pane or reading a legacy default creates none.

Ctrl/Cmd+Z and the existing redo chords retain global document undo while a non-editing panel
control or the rail has focus, subject to existing interaction guards. Do not swallow those
chords in generic panel/field handlers. A focused text input or active canvas text-edit session
keeps its native/local undo until editing ends; it must not simultaneously undo the document.
Shift+Enter/Space handling is specific to the Snapping tab, not a blanket key listener.

### 1.6 Multiple pages

The customer's six fractions remain 25%, 33.3%, 50%, 66.6%, 75%, 100% on both axes;
no invented 0% line or editable fraction list. The lead is asking whether the page targets
come from all visible pages or only the active page. **(Proposal; unanswered default A)**
use all visible pages; do not silently constrain snapping to the active page. This scope
choice does not introduce another control. Each page contributes lines at its own global
origin plus its width/height fraction, with marker extents belonging to that page. Update
this paragraph and the spec when the customer answers. The Page toggle remains singular.

## 2. The marker on the canvas

While a drag is snapped, the canvas shows what holds it: one 1-screen-px solid line along the
matched target and one small cross at the matched point of the moved feature. No text, no
tooltip, no label, no numbers.

| Part | Value |
|---|---|
| Colour | `--reference-mark` (`#E5484D`, `REFERENCE_MARK`) with a 1 px white casing on each side, so it reads on dark artwork and on the pasteboard (3.2:1 on `--canvas-bg`; the casing is the boundary that reaches 3:1). Same token as the reference marks: both mean "this exact place", and the blue selection and green guide keep their meanings. |
| Line | 1 device px, solid, constant screen width at every zoom; no dash, so it is not mistaken for a guide. |
| Line length | The target's own extent plus 8 screen px on each side: the matched edge segment of an object's box, the full length of the matched guide line, or the full target page side for a page fraction (a vertical line runs from page.y to page.y + height; a horizontal line from page.x to page.x + width). |
| Cross | 12 x 12 screen px, 1 px arms (a plus), white casing, centred on the matched point of the moving feature: the corner, edge midpoint, centre, anchor, or the dragged edge's own intersection with the line. |
| Both axes | Two lines (one vertical, one horizontal) and one cross at their intersection. A node target that snaps both axes draws the same. |
| Layer order | Above the artwork, the blue live preview, the axis lines and the reference box; below the selection/group box, the handles and the DOM overlays. The marker is never interactive (no hit area, no cursor change) and never captures the pointer. |
| Life | Appears in the frame the target captures, follows the pointer while it holds, disappears in the frame the snap releases, on pointer-up, on Escape, on a tool change and when the drag is abandoned. No fade, no animation (it must not lag the drag). |
| Reduced motion | Nothing to reduce; the marker has no transition. |

For a page fraction the line is drawn across the page even where the page is outside the
viewport (the draw list clips to the viewport, as every decoration does). For an object target
the line does not extend across the whole canvas: the maker must see *which* edge matched.

## 3. Behaviour the canvas must feel

- **Zones.** Capture and release are screen pixels converted with the current zoom at the
  moment of the frame. Zooming during a drag (wheel or pinch) keeps the same feel.
- **Hysteresis.** A new target captures when the raw, unsnapped moving point is within
  Capture of it. It holds until the raw point is more than Release from the same target; while
  it holds, no other target on that axis can take over (`adrs.md` decision 4). The snap never
  switches at one pixel, and a neighbouring line cannot steal a held snap by being a hair
  nearer.
- **Raw pointer.** The distance is measured from the position the pointer would produce
  without snapping, never from the snapped position. A slow sweep therefore crosses and
  leaves a target once, and a fast sweep cannot "stick".
- **Ties.** Nearest wins; on an exact tie nodes before guides before object boxes before page
  fractions. Ties are exact equality, not "within a pixel"; a near tie is a near win.
- **Axis lock.** With Shift held for a horizontal or vertical move, only the free axis snaps;
  the locked axis shows no line. Ctrl alone still copies (its meaning in the Select tool) and
  established moves and all supported creation phases read Alt as temporary inversion
  (§10–11), including with Ctrl held. Other tool scopes retain existing modifier rules.
  Enabled document axes further constrain the free-axis set.
- **What moves.** A multi-selection or a group snaps by the box of the whole selection, not by
  each member: its four edges, two centre lines and nine reference points. A resize snaps the
  moving edges and the dragged reference point; the fixed edges do not pull.
- **Rotate and skew** do not snap (rotation keeps its 22.5° stops). A rotated object still
  snaps by its axis-aligned box edges during a move or resize, because that is the box the
  Select tool shows.
- **Node tool.** Anchors snap to every enabled target. Handles snap to nodes only, so a handle
  can be made exactly as long as a neighbouring node offset; they never snap to boxes, guides
  or page lines. The edited path's own other anchors are targets, the dragged anchor is not.
- **Creation.** Rectangle, Ellipse and Polygon/Star show snapping at the prospective
  creation anchor before a press, on their first click, and during the creation drag or
  placement preview (§10). The existing size readout shows the snapped size.
- **Pen.** The first click and every placed node snap; the rubber-band preview follows the
  snapped point, so the segment the maker sees is the segment that is committed.
- **Targets.** All effectively visible leaves except the dragged object(s): locked objects
  included (they are often the reference), hidden objects excluded, guides included as lines
  and anchor points, linked clones by their own box and nodes. The selection itself and every
  other object being dragged contribute nothing.
- **Empty state.** Page lines exist in an empty document, so a newly drawn shape can be put on
  the page's 25 % line without another object. There is no 0 % line (customer's list); the
  left and top page edges are not snap targets of their own.
- **Drawing hover and first click.** Creation and Pen use the prospective anchor and
  show/commit the same eligible snapped point without requiring a held mouse button (§10).
  Select hover and plain object-selection clicks do not snap or move objects. A nudge
  with the arrow keys, a typed move and a typed size remain exact input and never snap.
- **One history step.** A snapped drag commits exactly what an unsnapped drag commits, under
  the same label. The settings changes are their own steps (`set_document_snapping`). The snap
  state, the marker and the held target are never written to the document.
- **Performance.** The target list is built once at the press from the read the session
  already holds; each move event scans it without reading the document. Nothing is rebuilt
  when a snap releases.

## 4. Feedback, states and refusal

- No status-bar text, no toast and no live-region message for a snap: it would fire on every
  frame. The marker is the feedback.
- The cursor does not change over a target. Drawing hover uses the same snapping marker
  as placement; ordinary Select hover gets no extra snapping highlight.
- Invalid field text is refused with the validation chip and writes nothing; refused
  or interaction-gated commands retain their stored values.
- Turning snapping off during a drag is not possible through panel controls (settings are inert during
  a canvas drag); temporary move inversion follows §10 and changes feedback in the same
  frame. Creation now uses the same inversion (§11); unrelated tool scopes retain
  their existing Ctrl+Alt suspension.
- Turning the document's switch off while a target is held releases it and removes the marker;
  the drag continues unsnapped.

## 5. Texts

| Where | Text |
|---|---|
| Rail tooltip | "Snapping — on" / "Snapping — off"; shortcut line as §1.3 |
| Target tooltips | "Snap to the boxes of other objects." / "Snap to guides." / "Snap to nodes." / "Snap to the page's 25 %, 33.3 %, 50 %, 66.6 %, 75 % and 100 % lines." |
| Capture tooltip | "How close, in screen pixels, a point must come before it is pulled onto a target." |
| Release tooltip | "How far, in screen pixels, it must be pulled away before it lets go. Never smaller than Capture." |
| Targets group | "Which things a moved point is pulled onto." |
| Capture refusal | "Enter a non-negative number" |
| Release refusal | "Enter a non-negative number" |

No German strings in the UI (the app is English); this table is the English copy.

## 6. Acceptance map

| Criterion | Where this file fixes it |
|---|---|
| 1 | Defaults above; §1.4–1.5, stored values are the only source |
| 2, latest customer placement | §1.2–1.3 |
| 3, 4 | §3 "Zones", "Hysteresis", "Raw pointer" |
| 5 | §1.4, §1.6 (target switches/page scope), §3 "Targets", "Empty state" |
| 6 | §3 "Ties" |
| 7 | §3 "What moves", "Axis lock", "Node tool", "Creation drags", "Pen" |
| 8 | §2 |
| 9 | Defaults above, §3 "Axis lock", §4 |
| 10, latest customer placement | §1.4–1.5 |
| 11 | §3 "Performance" |
| 12 | §3 "One history step", §2 "Life" |
| 13 | §3 "Targets" |
| 14 | Defaults above, §1.5, no opening write |
| 15 | §2 "Line length", §3 "Empty state" |

## 7. Revised UI review checklist

Before final APPROVE, verify all four enabled/selected combinations with the panel expanded
and collapsed; plain activation versus Shift-click and Shift+Enter/Space; no double toggle
or transient pane change; sticky Snapping across selection/tools; explicit switch and target
editing; zone refusal, normalization and Escape; document undo from rail/button focus and
native/local undo from typing contexts; focus rings, tab order, coarse-pointer access and
panel fit at the existing minimum window size. Confirm no narrow bar remains above headers.
Check all-visible versus active-page targets against the customer's eventual answer. The
existing marker and performance criteria remain required; this source review did not run them.

## 8. Partial implementation review — 2026-10-30

**Source review only; final UI acceptance pending.** Reviewed the revised `PanelStrip`,
`PropertiesPanel`, `SnappingSection`, `useSnapping`, the shared `EntryField`/`PanelSwitch`,
global `useHistory` handling and bounded Rust panel routing. The separate narrow toolbar
has been replaced by the rail entry and dedicated pane. Enabled indication is separate
from `aria-selected`; the pane exposes the switch, four target toggles and both zone fields.
Setting commands call the editor's synchronization path. The UI parser accepts 0, finite
fractional values and values above 100, rejecting negative/non-finite/empty input.

The installed Radix Tabs implementation composes the supplied mouse-down/key handlers
before its internal activation handlers, respecting prevented events. Thus the revised
early Shift mouse-down interception is in the correct location. The repeated-key case
was reported to the implementer: Shift+Enter/Space must suppress defaults on every keydown
but invoke the toggle only when `event.repeat` is false. This prevents a held key creating
multiple history entries. Recheck this fix and synthetic-click suppression in the browser.

The Rust selection observer changes only Document/Style automatically, so Snapping retains
its selection-independent pane identity. Global history handling skips text-entry targets;
the new field handlers do not swallow native/local text undo. The existing field restores
on Escape and follows stored values outside editing. Runtime release normalization,
collapsed-pane preservation, four visual states, real focus transitions and undo refresh
still require the implementer's/tester's checks. No build, test or browser was run by this
reviewer, and no PASS or final APPROVE is claimed here.

## 9. Final UX review — 2026-10-30

**APPROVE.** The earlier repeated-key finding is fixed in current source: Shift+Enter/Space
suppresses default handling on every keydown and toggles only when `event.repeat` is false.
Reviewed the current rail and settings source plus the saved Chromium screenshots
`target/browser-check/snapping-initial.png` and `target/browser-check/snapping-verified.png`.
No remaining concrete blocker was found against the amended rail/panel UX contract.

The screenshots show the dedicated Snapping pane, readable controls and px suffixes, no
narrow toolbar above its header, distinct explicit on/off switch states, independent target
states, and the enabled check badge separate from the strong selected-tab appearance.
The off/open screenshot correctly keeps the tab selected while the main switch is off;
the on/open screenshot adds the enabled badge and shows normalized Capture/Release values.
Source retains the existing inset keyboard focus treatment and tab semantics, with an
independent On/Off accessible description; the switch and target controls expose their
own checked/pressed states. Unselected/collapsed enabled appearance is defined in source
with editor tokens, rather than reusing selection state.

The lead reports the real Chromium smoke script `frontend/tests/browser/snapping.cjs`
passing plain and Shift clicks on the icon child, collapsed-pane preservation, modified
keyboard activation/repeat suppression, all four target switches, 0/fractional/123.5px
zone entry, release normalization, negative-value refusal, Escape, native field undo,
document undo/redo and sticky pane behaviour after drawing and tool changes. This is
lead-run browser evidence, not an independently repeated test by the UX reviewer.

This reviewer ran no builds, tests or live browser. The two screenshots cover expanded
off/on states, not every focus/hover/coarse-pointer combination; those states were inspected
in source. This approval covers the amended UI, not a fresh performance measurement,
snapping-engine correctness proof or the tester's full quality gate. Customer acceptance
remains outstanding.

## 10. PR #225 customer feedback: axes, move inversion and drawing preview

Authority: the lead relayed new customer feedback requesting two independent horizontal/
vertical controls, Alt temporarily inverting snapping while moving, and snapping feedback
while drawing before the first press, on the first click and along the Pen rubber band.
This section supersedes earlier no-hover and move-modifier wording. It leaves the accepted
Shift-click rail toggle, its keyboard equivalents, focus rules and undo ownership intact.
The coordinate-axis default below remains the working meaning. The customer has since
explicitly resolved broader creation-modifier scope in §11; preserve unrelated tool gestures.

### 10.1 Independent coordinate axes

**(Proposal; default pending the lead's directional question)** use `Horizontal (X)` and
`Vertical (Y)` under `Snap axes`, two independent pressed-state controls in a two-column
row after Snap targets and before Capture. Horizontal means correction of the x coordinate
(left/right movement, generally to a vertical target line); Vertical means correction of y
(up/down movement, generally to a horizontal target line). Do not label them only by target
line orientation. Accessible names contain the visible axis labels.

Both defaults are on, including legacy files without these flags. Values belong to the
document and each edit is one undoable setting step. They remain editable when global
snapping is off and do not enable it implicitly. Both off is valid: no correction or marker,
while the global switch and blue rail indication continue to show the stored global setting.
Turning an axis off clears any held component on that axis. A point target must not secretly
restore a disabled coordinate: only eligible coordinate components may be corrected.
For a one-axis point snap, the target must already lie on the reachable coordinate line
within the architect's geometric tolerance; do not project an unreachable point into a
false match. Curve targets must intersect the permitted motion line. The ADR owns exact
geometry/tolerance; the marker must identify a target the moving point actually reaches.
Shift's movement constraint is intersected with the enabled axes. Preserve exact typed input,
current tool geometry constraints and the existing capture/release zones.

### 10.2 Alt during a move, with existing gestures preserved

For the established Select-move scope (creation is additionally covered by §11), once a Select
object-move or copy-move has begun, holding Alt uses the opposite of the stored global
snapping-enabled value. Off becomes temporarily on; on becomes temporarily off. Target kind
and axis choices are unchanged, so Alt never enables a disabled target or coordinate. Ctrl-only
copy and Shift axis lock retain their meanings, including the existing `ctrl && !alt` copy
guard: Alt must not accidentally turn a move into a copy. Inside this established move, Alt inversion
also applies with Ctrl held; do not additionally apply the old Ctrl+Alt suppression there.
Every supported creation phase also uses Alt inversion as explicitly required in §11.
Other tool scopes retain their existing Ctrl+Alt rule.

No register, history entry, dirty state, panel switch, rail badge or panel visibility changes
on Alt press/release. Re-evaluate from the raw pointer-derived position when the modifier
changes, even without pointer movement. Losing effective snapping immediately clears the
held target/marker; re-enabling it resolves eligible targets afresh. The movement remains
one normal Move/Copy history operation on release. Release, Escape, abandoned capture and
window focus loss clear transient state; do not leave inversion latched for the next gesture.

Bounded source conflict review: `curvyo-ui-core/src/select_tool/press.rs` and `select_tool.rs`
make Alt at pointer-down arm lasso before body/handle hits; Alt-click also cycles overlapped
objects. `curvyo-editor-wasm/src/session/offset.rs` uses Alt during a corner drag to cycle
corner style. Preserve Alt-at-press selection/lasso and Offset behaviour. The safe Select
move gesture is start moving, then hold Alt. The prior Pen Ctrl/Alt coupling is now subject
to the explicit creation clarification in §11; Alt must invert snapping without changing
the Ctrl geometry constraint. No reassignment for unrelated tools is authorized.

### 10.3 Drawing hover, first click and Pen preview

Rectangle, Ellipse, Polygon/Star and Pen resolve the prospective creation anchor while the
pointer moves over the canvas, before any mouse button is held. With eligible snapping on,
show the existing line/cross marker for the resolved anchor. The raw pointer remains the
source for capture/release distances; hover must not synthesize document edits or an object.
The first click uses that same eligible resolution, including a click with no preceding
pointer move. A drag creation's initial anchor and the marker therefore agree.

For simple click/default-sized shape placement, snap the existing placement anchor/origin,
respecting enabled coordinate components; preserve the tool's default dimensions, angle and
anchor convention. Do not resize the default shape to make its far edge match another target.
For an existing click-move-click placement, keep its current constrained preview and apply
snapping only to eligible free coordinates. A geometry constraint must never be broken to
satisfy a disabled/ineligible axis. Typed dimensions and coordinates remain exact.

For Pen, both the prospective first node and the endpoint of the rubber band resolve while
no button is held. The click commits the previewed eligible endpoint. Existing path
continue/join/close precedence and angular constraints still win; snap candidates must be
compatible with them. The in-progress endpoint, handles and generated drawing preview are
not their own targets. Previously eligible established anchors retain their existing rules;
this feedback does not expand self-snapping targets.

Marker lifecycle: appears only when an eligible prospective point is captured; updates on
pointer movement or relevant modifier/setting changes; vanishes immediately when snapping
is effectively off, both axes are off, the target releases, the pointer leaves the canvas,
a tool changes, the operation is cancelled or capture/window focus is lost. Pointer-up ends
an active drag's marker; a subsequent drawing hover/placement update can show the next
prospective anchor. Do not leave the previous drag's marker stuck after release. Pen finish
and Escape clear its held target before any new first-node preview. Moving into the panel
clears drawing feedback; editing settings does not create artwork. No labels, animation or
new cursor are added.

Reuse the existing eligible-target index across consecutive hover/preview events when the
document/visibility context has not changed. Invalidate/rebuild on actual target changes,
page geometry, settings or editing-context changes, not on every pointer event or every
preview endpoint. Use the same eligible target set when a hover becomes a press/drag so
there is no feedback jump; exclude the live moving/drawing preview. The architect and
implementer own cache identity and geometry policy; these UX notes require consistent
feedback without a fresh document read per hover frame.

### 10.4 Review evidence still needed for this feedback

Recheck X-only, Y-only, both and neither; point targets and constrained drawing; defaults
and saved/reopened flags; one-step setting undo. Demonstrate stored-off/Alt-on and
stored-on/Alt-off during a move, restoration without pointer motion, Ctrl copy and Shift
constraint, cancellation/focus loss, unchanged rail/panel state and preserved Alt lasso/
click cycling and Offset corner style. Demonstrate hover capture before first press,
first-click agreement, default-sized shape anchor, click-move-click preview, Pen first node
and rubber band, preview self-exclusion and marker cleanup. The §9 approval is historical;
these additions need their own implementation and validation evidence below.

### 10.5 Historical partial source review before the creation-Alt extension

Read-only review, 2026-10-30: current `SnappingSection` exposes independent `Horizontal (X)`
and `Vertical (Y)` pressed-state buttons under `Snap axes`, before zone fields. Their visible
labels supply accessible names, keyboard clicks retain focus, pointer clicks return to the
canvas, and each uses the dedicated axis setter plus editor synchronization. The hook's
initial defaults enable both. The WASM settings bridge preserves the current axis flags
when other targets/zones change; the axis setter preserves the remaining document settings.
No new generic key handler intercepts native field undo or the existing document undo path.

The rail/panel switch continues to read the stored enabled value, independently of the
effective move result. Current session code uses stored-enabled XOR held-Alt during an
established Select move and returns no effective snapping when both axes are off. The
marker renderer now consults effective snapping rather than the stored flag, allowing a
marker when Alt temporarily enables a stored-off move. Outside moves, the existing Ctrl+Alt
suppression remains. These are source observations, not a browser demonstration.

Drawing/Pen hover routes resolve without a pressed button. The toolbar's `hasDrag` derives
from an actual button/operation, so an idle cached hover context alone does not disable the
settings controls. Pointer leave without a held button and setting changes clear the snap
context; modifier changes refresh drawing/Pen at the retained pointer position. The drawing
context reuses the current document version/exclusion context. No concrete new UI blocker
was found in this bounded source review. Final review still needs saved screenshots and
browser evidence for axes, both directions of temporary inversion and idle marker lifecycle;
no visual PASS/APPROVE, build or test is claimed for this feedback yet.

### 10.6 Historical saved-screenshot review and feedback approval before §11

Initial finding: **CHANGES REQUESTED**, subsequently resolved below. Reviewed saved screenshots
`axes-{both,x,y,none}-hover.png` and `alt-{true,false}-{false,true}-held.png` in
`target/browser-check`. All four axis combinations read clearly: X permits a vertical
target line with the cross at the permitted x coordinate; Y permits a horizontal target
line; both show both lines; neither shows no marker while the global switch remains on.
The markers are visible before pressing a mouse button. The independent axis controls fit
the existing pane and use its established pressed-state appearance.

The Alt screenshots show the intended stored-versus-effective distinction: stored-on
retains the panel switch and rail state while its snap marker disappears; stored-off
retains its off switch while temporary snapping shows a marker. However,
`alt-false-true-held.png` also displays `Copy` and a plus badge. The lead confirmed this is
a real feedback defect, not screenshot timing: move indicator rendering reads Ctrl without
the existing `!alt` guard, while preview/commit correctly avoid copying. The readout and
badge must use the same effective copy condition as the operation before final approval.
The tester/implementer own the focused regression and correction; this reviewer ran none.

The lead reports `frontend/tests/browser/snapping-feedback.cjs` passing exact saved rectangle
coordinates and persisted axis flags in all four combinations, axis undo/redo, drawing
hover before a press, second-click and Enter placement, both directions of moving Alt
inversion including Ctrl without an extra committed copy or stored-setting mutation, and
no JavaScript errors. The earlier rail/panel browser smoke also passed according to the
lead. These runtime outcomes are attributed evidence, not independent reviewer reruns.

The initial scratch placement screenshot showed an uncommitted live preview and intentional
first-field focus. Synchronized browser interaction (two animation frames before the second
canvas click) demonstrated successful placement, as did Enter. That scratch result is not
a product defect and does not indicate incorrect snapping geometry. Final new-feedback UX
approval remains pending corrected Copy feedback and updated screenshot evidence; no
fresh live browser, build or tests were run by this reviewer.

**Final verdict: APPROVE (2026-10-30).** Inspected the freshly regenerated
`target/browser-check/alt-false-true-held.png` after the tester rebuilt WASM and reran the
browser checks. The move readout now contains only the displacement, with no `Copy` text
or plus badge. Temporary snap lines remain visible while the stored Snapping switch stays
off, so the corrected feedback agrees with the actual move and document state. This
resolves the only outstanding visual finding; the four axis/hover screenshots reviewed
above remain consistent with the requested coordinate semantics.

The lead reports the tester's refreshed browser checks passing with explicit assertions
that neither Copy text nor the plus badge exists before capture, together with all axis,
click/Enter placement and Alt saved-geometry checks. The lead also reports 31 public
acceptance tests and the full gate passing, with 9,519/9,519 workspace tests green. These
are attributed tester results, not tests rerun by the UX reviewer. This reviewer inspected
the saved final screenshot only; no live browser, build, test or production edit was made.
No concrete amended-UX blocker remains. Customer acceptance is still a separate step.

## 11. Explicit customer clarification: Alt inversion throughout creation

**Customer authority, relayed by the lead on 2026-10-30:** Alt temporarily inverts stored
snapping during **all** creation phases of Pen, Rectangle, Ellipse and Polygon/Star, as well
as the existing Select-move scope. This resolves the previous move-only default; it is not
an optional proposal. Specification criteria 9, 17 and 18 carry the same clarification.
The existing rail Shift-click/Shift+Enter/Shift+Space interactions, focus behaviour and
settings history remain unchanged.

### 11.1 Effective state and geometry

In every supported creation phase, effective snapping is the stored enabled value XOR
held Alt: stored-off becomes temporarily on; stored-on becomes temporarily off. Ctrl+Alt
uses the same inversion; do not layer the old Ctrl+Alt force-off rule over it. This applies
before the first press, during idle drawing hover, on the first press/click, while a simple
click placement follows without a held button, during a button-down creation drag, and to
the Pen's next-point/rubber-band preview and each placed point. No-button previews must not
be mistaken for unsupported interactions merely because there is no drag flag.

Axis and target-kind choices remain authoritative; Alt does not enable a disabled axis or
kind. Both axes off still produces no correction/marker. The panel switch, rail blue state
and badge continue to describe the stored document setting, not effective temporary state.
Alt press/release creates no register write, dirty change or history step. Repeated Alt
keydown must not toggle or latch anything; held-state inversion lasts only until release.

Pressing or releasing Alt with an unmoved pointer updates the eligible preview and marker
immediately from the raw pointer-derived point. Re-resolve the free endpoint; do not reuse a
previously snapped endpoint as raw input. Clear held snaps when inversion disables snapping
and resolve afresh when it enables it. Already fixed shape start anchors and already placed
Pen anchors never move. Frozen typed size/placement remains exact and does not reflow on Alt;
its fixed geometry must not acquire a misleading marker suggesting a different placement.
Unfrozen simple-click placement follows the same effective state as hover and drag creation.

Alt changes snapping only. Ctrl's current creation geometry constraint (shape proportions,
polygon/star constraint or Pen angle constraint) and Shift's current creation meaning remain
in force when combined with Alt. Candidate snaps must satisfy those constraints and the
enabled axes; no incompatible target may bend the constrained geometry. Architecture owns
the exact constrained resolver and Pen Ctrl-coupling correction. No extra product decision
is needed to implement the customer's explicit creation request.

### 11.2 Feedback, completion and cancellation

Use the existing thin line/cross marker only, with the same appearance and eligible-target
index reuse as §10.3. At the same pointer location, Alt can remove or reveal it before a
button is pressed; the next click commits the point that the current preview indicates.
No new status label, toast, cursor or temporary blue rail state is added.

Pointer leave, cancellation, tool switch and focus/capture loss clear obsolete marker and
held-target state according to the existing lifecycle. A later drawing hover resolves from
the current held modifiers; no inversion remains latched. Ending a drawing gesture clears
its transient state without modifying stored settings. Select Alt-at-press lasso/cycling
and Offset corner-style Alt remain unchanged; this extension is limited to the named
creation tools and the already accepted established Select move.

### 11.3 Approval boundary and required evidence

The earlier APPROVE verdicts covered the previous rail/axes/move implementation. They are
historical and do not approve this extension. Before a fresh verdict, demonstrate both
stored-enabled values with Alt held/released at a stationary pointer in every creation
phase: hover/prepress, first click, simple-click follow, button-down drag, Pen first point
and rubber band. Include Ctrl-constrained drawing with Alt, one-axis and both-off settings,
fixed first/placed anchors, frozen typed geometry, unchanged document switch/rail state,
cleanup and committed-coordinate agreement with the last preview. This amendment is
UX documentation only; this reviewer ran no live browser, build or test for it.

### 11.4 Final creation-Alt UX review

**APPROVE**, based on current bounded source inspection, fresh saved screenshots and the
lead's attributed browser evidence. Reviewed `creation-rectangle-{false,true}-alt-held.png`,
`creation-follow-click-{false,true}-alt-held.png` and `creation-pen-false-alt-held.png` under
`target/browser-check`. In the stored-off cases Alt reveals the snapping lines while the
panel switch remains off; in stored-on cases it removes the lines while the switch remains
on. The selected rail entry and enabled badge continue to represent document/pane state.
The focused placement fields remain readable with a visible focus indicator; their live
dimensions and the preview agree with the effective snapping state in these captures.
Pen feedback is also visible before the first press without a held mouse button.

The focused-chip event defect is resolved: `PlacementChip` now returns early for Alt so
central modifier/menu handling receives it, while ordinary editing keys retain the chip's
existing handling. `PenEntryChip` already forwards modifier keys. This bounded source check
supports the focused-follow interaction without claiming a new native text-undo test.

The lead reports the refreshed creation browser script passing all supported tool hovers,
both stored-enabled values, Alt before the first click, button-down creation, no-button
following placement, and press-to-placement fixed anchors. The reported checks include
stationary Alt press/release changing and restoring live dimensions without pointer motion,
central Alt/menu prevention, exact saved geometry, unchanged stored flags and no JavaScript
errors. These checks were run by the lead, not independently repeated by this reviewer.

No concrete blocker remains for the clarified creation-phase UX. The screenshots are
representative states rather than every possible modifier/tool combination; geometry and
independent public acceptance/full-gate checks remain the tester's responsibility and were
still pending at this review. This reviewer ran no live browser, build or tests and made no
production edits. This is a UX approval, not an overall test PASS or customer acceptance.
