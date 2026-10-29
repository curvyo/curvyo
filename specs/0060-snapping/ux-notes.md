# 0060 Snapping: UX notes

Status: Ready for build, 2026-10-29. Owner: ux-engineer. Source: the customer's request of
2026-10-17 and amendment of 2026-10-29 ("snapping ist eine property vom document. eine page hat
snapping punkte bei 25%, 33.3% 50% 66.6% 75% 100% sowohl horizental als auch vertikal"), the
criteria in `specification.md`, and the rows "Select bar layout", "Guide (`0046`)", "Reference
box", "Reference mark", "Background block", "Properties panel: Document section" and "Switch"
of `docs/design-system.md`. Where this file and a criterion differ, the criterion wins and this
file is corrected.

Defaults taken from the spec's open questions (customer may change): snapping off, all four
target kinds on, capture 8 px, release 14 px, Ctrl+Alt turns it off while held, block in the
Document tab. The first three are stored in the document from the first edit; the modifier is
not stored.

## 1. The two places

### 1.1 Select bar: the Snapping toggle

One icon-only toggle, **last group of the Select bar**, after "Linked clone" and the Trace
button, in every selection state and in the Select tool only. It is appended last so no
existing control moves (`docs/design-system.md`: "a new control is appended at the bottom of
its group"; here: of the bar). A divider is drawn to its left only when it shares a row with
another group, as for every group.

| Property | Value |
|---|---|
| Size and shape | 28 x 28 px, `rounded-md`, like the Flip buttons and the Guide button |
| Glyph | Lucide `Magnet`, 16 px, 1.5 px stroke, `absoluteStrokeWidth`, `currentColor` (`--toolbar-icon`) |
| Ground | unpressed: none; hover `--editor-accent-hover`; pressed (`aria-pressed`) `--toolbar-icon-active-bg` / `-fg`; focus-visible 2 px `--editor-accent` ring with 1 px `--toolbar-bg` offset |
| Accessible name | "Snapping" (the tooltip's first line, WCAG 2.5.3) |
| Tooltip | Radix, 400 ms, `side="bottom"`, 260 px: line 1 semibold "Snapping"; line 2 `--panel-muted-fg` "Pull moved points onto edges, guides, nodes and page lines." No key hint: there is no shortcut (Ctrl+Alt only turns it off while held, which is not a toggle). |
| State | Reads and writes the document's `snapping_enabled`. Pressed exactly when it is on. Never disabled, never dimmed, never hidden with nothing selected: it is a setting, not a command on the selection. |
| Press | One press, one commit `set_document_snapping`. Mouse press returns focus to the canvas; Space or Enter keeps it on the button. |
| Announcement | No extra live region. `aria-pressed` carries the state; the panel's switches say the same. |

The two setting switches "Scale stroke width" and "Scale corner radius" keep their place at
the start and are not joined by a third switch: a third labelled switch would push the first
row past 324 px at 800 x 600 and wrap the settings, which the layout row forbids. The
snapping toggle belongs to the document, not to the selection, so it can sit at the end
without being read as a selection command.

**Measured at build**: not yet. Expected at 800 x 600 with one rectangle selected: the bar
grows from 6 rows (208 px) to 7 (252 px) only while the new group does not fit an existing
row; at 1280 x 800 it must stay 2 rows (72 px). The UX review measures and records the exact
figures.

### 1.2 Document tab: the Snapping block

A block of its own, **after "Fit to content" and before the Background block** (so the size
group stays together, and Background remains the last block as its row requires). A 16 px gap
and a 1 px `--toolbar-icon` at 25 % rule separate it from Fit; the Background rule stays
between this block and Background. The block is in the tree in every document state,
including with no objects, and whatever the active tool.

Title row, 28 px: **"Snapping"**, 12 px semibold `--toolbar-icon`, at the left. No on/off
control here: the Select bar owns the switch (the customer asked for it "im Select tool"). The
block therefore configures snapping whether it is on or off; no row is disabled or dimmed.

| Row | Control |
|---|---|
| Targets | A `role="group"` named "Snap targets" of four independent toggle buttons with `aria-pressed`, 84 x 28 px each, 4 px apart, in a 2 x 2 grid (two rows, 28 px, 4 px apart, 64 px): **Objects**, **Guides**, **Nodes**, **Page**. Each label is 14 px, the group has a frame as the Style `ToggleGroup` (1 px `--toolbar-icon` at 60 %, `rounded-[5px]`, 1 px dividers); pressed uses `--toolbar-icon-active-bg` / `-fg`, hover `--editor-accent-hover`, focus 2 px `--editor-accent` inside the frame. Tooltip, one line each: "Snap to the boxes of other objects.", "Snap to guides.", "Snap to nodes.", "Snap to the page's 25 %, 33.3 %, 50 %, 66.6 %, 75 % and 100 % lines." A press writes that one flag and is one commit; with a mouse the focus returns to the canvas, with Space or Enter it stays on the button. |
| Capture | "Capture" in the label column, a `TextField` 96 px in the control column, right-aligned 14 px tabular text, fixed "px" suffix inside the right edge, whole numbers 1 to 100. Enter commits and returns focus to the canvas; Tab commits and moves on; leaving the field with edited text commits it; Escape restores; Enter on unedited text writes nothing. Below 1 or above 100: the validation chip "Enter a whole number from 1 to 100" (2 px `--field-invalid`, `aria-invalid`), the value is not written. |
| Release | Same field, same limits, tooltip explains the hysteresis. A value below the stored capture is raised to it on commit and the field then shows the raised number; the same happens on open (criterion 3). Changing Capture upward therefore also raises Release in the same commit and both fields re-read. |

Tooltips (400 ms, `side="left"`, text only): Capture "How close, in screen pixels, a point must come before it is pulled onto a target."; Release "How far, in screen pixels, it must be pulled away before it lets go. Never smaller than Capture."; Targets group "Which things a moved point is pulled onto.".

Block height at 244 px content: 28 (title) + 8 + 64 (targets) + 8 + 28 (Capture) + 8 + 28
(Release) = 172 px, plus the 16 px gap and rule above it. The panel scrolls as one; the block
never scrolls inside itself and the title row never moves when a value is refused.

Keyboard order: title, Objects, Guides, Nodes, Page, Capture, Release. With a mouse, every
control returns focus to the canvas (the panel's rule); with a key it stays. `Shift+Ctrl+D`
still focuses the first control of the section (Width), not this block.

## 2. The marker on the canvas

While a drag is snapped, the canvas shows what holds it: one 1-screen-px solid line along the
matched target and one small cross at the matched point of the moved feature. No text, no
tooltip, no label, no numbers.

| Part | Value |
|---|---|
| Colour | `--reference-mark` (`#E5484D`, `REFERENCE_MARK`) with a 1 px white casing on each side, so it reads on dark artwork and on the pasteboard (3.2:1 on `--canvas-bg`; the casing is the boundary that reaches 3:1). Same token as the reference marks: both mean "this exact place", and the blue selection and green guide keep their meanings. |
| Line | 1 device px, solid, constant screen width at every zoom; no dash, so it is not mistaken for a guide. |
| Line length | The target's own extent plus 8 screen px on each side: the matched edge segment of an object's box, the full length of the matched guide line, or the full page side for a page fraction (from x = 0 to x = document width for a vertical line, the same for a horizontal one). |
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
- A setting write never fails from the UI: the fields refuse out-of-range text with the
  validation chip and write nothing; the switches always write.
- Turning snapping off during a drag is not possible with the toggle (the bar is inert during
  a canvas drag); Ctrl+Alt is the way to suspend it, and it takes effect in the same frame.
- Turning the document's switch off while a target is held releases it and removes the marker;
  the drag continues unsnapped.

## 5. Texts

| Where | Text |
|---|---|
| Toggle tooltip | "Snapping" / "Pull moved points onto edges, guides, nodes and page lines." |
| Target tooltips | "Snap to the boxes of other objects." / "Snap to guides." / "Snap to nodes." / "Snap to the page's 25 %, 33.3 %, 50 %, 66.6 %, 75 % and 100 % lines." |
| Capture tooltip | "How close, in screen pixels, a point must come before it is pulled onto a target." |
| Release tooltip | "How far, in screen pixels, it must be pulled away before it lets go. Never smaller than Capture." |
| Targets group | "Which things a moved point is pulled onto." |
| Capture refusal | "Enter a whole number from 1 to 100" |
| Release refusal | "Enter a whole number from 1 to 100" |

No German strings in the UI (the app is English); this table is the English copy.

## 6. Acceptance map

| Criterion | Where this file fixes it |
|---|---|
| 1 | §1.2 (defaults are read, not shown); the stored values are the only source |
| 2 | §1.1 |
| 3, 4 | §3 "Zones", "Hysteresis", "Raw pointer" |
| 5 | §1.2 (target switches), §3 "Targets", "Empty state" |
| 6 | §3 "Ties" |
| 7 | §3 "What moves", "Axis lock", "Node tool", "Creation drags", "Pen" |
| 8 | §2 |
| 9 | §1.1 tooltip (no key), §3 "Axis lock", §4 |
| 10 | §1.2 |
| 11 | §3 "Performance" |
| 12 | §3 "One history step", §2 "Life" |
| 13 | §3 "Targets" |
| 14 | §1.2 (defaults), no UI effect |
| 15 | §2 "Line length", §3 "Empty state" |
