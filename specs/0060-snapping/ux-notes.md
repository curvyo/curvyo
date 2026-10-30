# 0060 Snapping: UX notes

Status: UX implementation review APPROVE, 2026-10-30; full quality gate and customer acceptance remain separate.
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
than capture. Ctrl+Alt temporarily suspends snapping. These notes do not change that
modifier or introduce a new global shortcut. Settings remain document-owned and undoable;
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
  Canvas-drag gating remains as for other settings: no mid-drag panel mutation; Ctrl+Alt is
  the existing temporary suspension route.

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
indication stays visible. Temporary Ctrl+Alt suspension does not change the persisted enabled
indication; the marker disappears and the tooltip may say `Snapping on, suspended`.

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
| 3 | `Capture`: existing panel number-entry pattern, displayed in screen `px`, accepts finite nonnegative values. |
| 4 | `Release`: same; below Capture normalizes to Capture and shows the stored result after commit. Raising Capture beyond Release raises Release in the same document transaction. |

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
  Ctrl+Alt together turn snapping off, so a Ctrl copy does not lose snapping.
- **What moves.** A multi-selection or a group snaps by the box of the whole selection, not by
  each member: its four edges, two centre lines and nine reference points. A resize snaps the
  moving edges and the dragged reference point; the fixed edges do not pull.
- **Rotate and skew** do not snap (rotation keeps its 22.5° stops). A rotated object still
  snaps by its axis-aligned box edges during a move or resize, because that is the box the
  Select tool shows.
- **Node tool.** Anchors snap to every enabled target. Handles snap to nodes only, so a handle
  can be made exactly as long as a neighbouring node offset; they never snap to boxes, guides
  or page lines. The edited path's own other anchors are targets, the dragged anchor is not.
- **Creation drags.** Rectangle, Ellipse and Polygon/Star snap while the drag runs, including
  the first press (a creation drag has no "raw start" to preserve: the pressed corner is the
  raw point and snaps immediately). The existing size readout shows the snapped size.
- **Pen.** The first click and every placed node snap; the rubber-band preview follows the
  snapped point, so the segment the maker sees is the segment that is committed.
- **Targets.** All effectively visible leaves except the dragged object(s): locked objects
  included (they are often the reference), hidden objects excluded, guides included as lines
  and anchor points, linked clones by their own box and nodes. The selection itself and every
  other object being dragged contribute nothing.
- **Empty state.** Page lines exist in an empty document, so a newly drawn shape can be put on
  the page's 25 % line without another object. There is no 0 % line (customer's list); the
  left and top page edges are not snap targets of their own.
- **No snap on hover, no snap on plain click.** Only a drag (or a placement) pulls. A nudge
  with the arrow keys, a typed move and a typed size are exact input and never snap.
- **One history step.** A snapped drag commits exactly what an unsnapped drag commits, under
  the same label. The settings changes are their own steps (`set_document_snapping`). The snap
  state, the marker and the held target are never written to the document.
- **Performance.** The target list is built once at the press from the read the session
  already holds; each move event scans it without reading the document. Nothing is rebuilt
  when a snap releases.

## 4. Feedback, states and refusal

- No status-bar text, no toast and no live-region message for a snap: it would fire on every
  frame. The marker is the feedback.
- The cursor does not change over a target and there is no hover highlight.
- Invalid field text is refused with the validation chip and writes nothing; refused
  or interaction-gated commands retain their stored values.
- Turning snapping off during a drag is not possible through panel controls (settings are inert during
  a canvas drag); Ctrl+Alt is the way to suspend it, and it takes effect in the same frame.
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
