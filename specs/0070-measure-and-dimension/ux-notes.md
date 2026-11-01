# 0070 Measure and dimension: UX notes

Status: Draft for Ready. Owner: ux-engineer. Authority: `specification.md` (DIM-LINK A, DIM-LOOK A,
DIM-TEXT A; customer 2026-11-01: "nice, UX/UI consistent and above all working").
References: `docs/design-system.md`; `specs/0066-constraint-sketcher/ux-notes.md` ("0066 §n" below,
Revision 2 and the polish of 2026-11-01); `specs/0060-snapping/ux-notes.md` (marker, Alt);
`ToolRail.tsx`, `GuideButton.tsx`. **(Proposal)** marks reversible choices, vetoable at the demo.
Customer rule 2026-10-16: no operating tips or hint labels. Labels that name a control or a state
stay; there are no hint lines, no field tooltips. No browser or build was run for this document.

## Deviations from the specification (for the lead and the PO)

1. Criterion 2 allows "a hint line" in the top bar. The 2026-10-16 rule forbids it, and a bar is not
   needed: **Measure has no top bar at all** (as Sketch after 0066 Revision 2). The tool name is
   the rail tooltip and the pressed rail button; the kind is the mode label (§4).
2. Criterion 2 offers "Shift or a toggle". Decided: **Shift** (§5). No toggle control exists.
3. Criterion 6 gives Alt two jobs. They never overlap in time; resolved in §8.

## 1. Rail entry and icon

- **Place:** the **second button of the Path-tools card** (column A, below Offset; same one-active-tool
  rule, same look). Measure is a tool, so it cannot live in a command card. The card's accessible name
  changes from "Path tools" to "More tools"; Tab order and the roving toolbar model do not change.
- **Icon:** Lucide `RulerDimensionLine`, `size={20}`, same stroke as its siblings, `currentColor`.
  Distinct from Offset's glyph and from the Sketch Dimension ruler (different workbench).
- **Tooltip** (Radix, 400 ms, right): "Measure tool (M)", and "Measure tool (Esc, M)" while the Select
  tool has a selection (the letter M opens the move entry there, the same pattern as R). No hint line.
  Accessible name the same text. `aria-pressed` while active.
- **Letter M** (Inkscape's Measure key), a tool letter in every state except Select-with-selection,
  ignored in text fields and chips as the other letters. Sketch assigns no letter (0066 §2): unaffected.
- **Budget:** column A grows by 44px (card 92px) and ends 580px below the viewport top. The 640px
  minimum gives 560px: 20px short. The existing rule applies (the column scrolls under Tools in the
  smallest window). This is the "further tool" decision the design system reserves for the customer:
  **Frage MEAS-RAIL.** A (default, recommended): as above. B: Measure becomes a Select-bar text button
  like Trace (reachable only with a selection, which defeats "measure anything"). C: Import joins the
  Path-tools card (an action in a tool card; rejected in 0058).

## 2. Anatomy of a gesture

No bar, no panel. Everything is on the canvas.

| Phase | Canvas shows |
|---|---|
| Idle, pointer on canvas | plain `crosshair` as the Rectangle tool; no chip until the first pick (customer, PR #243); hover highlight on a pickable target (§6) |
| Anchor A set | filled 6px dot at A (`--preview-new`, 1px white casing), kept until the end |
| A to B, pointer moving | measure line + readout chip (§3), or the green dimension preview with Shift (§5) |
| B set (Shift not held) | the measurement stays pinned: blue line, anchor dots, readout chip at B. Nothing stored |
| B set with Shift | placement phase: the dimension follows the pointer; the next click places it |
| Placed | one history step "Dimension"; tool stays armed with nothing picked |

**Input forms (all three produce the same picks):** press at A, drag, release at B (press-drag-release);
click A, click B; or mixed. A press without movement over 4px is a click. **A pinned measurement ends**
on the next press, Escape, a tool change, an undo or redo, any document edit; pan and zoom keep it
(it is anchored in document coordinates).

**Pick sources** (what a press can hit, primary button only: a secondary press does nothing, customer
feedback on PR #243), in priority: (1) a snap point (node, centre, midpoint, rectangle corner,
polygon or star vertex, and the four axis ends of a whole ellipse that is not a circle) within
the snapping tolerance of `0060`; (2) the outline of a circle, ellipse that is a circle, arc or rounded
corner (`0013`) within the 8px Select hit tolerance: a **curve pick**; (3) the body of a straight
segment or a straight primitive edge (rectangle side between its box corners, polygon or star
side between its vertices): a **segment pick**; (4) empty canvas: a **free point** (only while snapping is off or nothing
is near; a free point is not a dimension anchor, see §7). Hidden objects are not picked; locked
objects can be measured. Existing dimensions are not pick targets.

**Kind from the picks** (one smart tool, as the Dimension tool of 0066 §1; the mode label always names
the result before the commit):

| Picks | Kind |
|---|---|
| curve pick (circle, arc, rounded corner) | **Radius**; Alt during placement switches to **Diameter** (§8) |
| two points | **Aligned**, **Horizontal** or **Vertical** by the placement position (below) |
| one segment pick, then placement | **Aligned** length of that segment |
| segment pick, then a second, non-parallel segment pick | **Angle** (the sector under the pointer chooses which of the four angles) |

**Placement rule for two points (Proposal; SolidWorks-style).** Let R be the box around A and B and P the
pointer. P above or below R (x within R's x range) gives Horizontal; P left or right of R (y within R's
y range) gives Vertical; P inside R or in a corner quadrant gives Aligned. The mode label changes in the
same frame the region does; the dimension line stays parallel to its kind's axis and follows P at a
screen distance of at least 12px from the measured points. The rule is a pure function of (A, B, P) and
is unit-tested with its eight regions.

**Selection first** (criterion 5, "selection first, else tool" as 0066): with **exactly one** eligible
object selected (a circle or arc: Radius; a straight segment or two-node open path: Aligned length) and
the pointer over empty canvas, the dimension preview follows the pointer at once, and a click places
it; no picking step. A press on any pick source (above) starts a normal pick instead. While the pointer
is over a pick source the preview hides and the hover highlight shows. The selection does not change.
Keyboard route: with an eligible selection and the tool active, **Enter** places the dimension at its
default position (24px, the side as in the 0066 default offset rule, ranks as in 0066 §7).

## 3. Live readout while measuring (criterion 1)

The "Live transform readout" chip, unchanged surface: `--toolbar-bg`, `--toolbar-icon`, 12px, 4px
radius, no shadow, 12px up and right of the pointer (flips below and left at the canvas edge and clear
of the rail, as the move readout). Two lines, tabular figures, U+2212 for minus, **display unit
`0015`, at most 3 decimals, trailing zeros trimmed** (the 0066 polish; no field, no setting):

```text
12.5 mm, -45°        length, angle
Δ 8.839, -8.839 mm   x and y distance, Y down as the move readout
```

Angle: clockwise on screen from straight right, -180° to 180° (the design-system convention for
the polygon/star angle). Radius: one line, "R 25 mm" (second line "⌀ 50 mm" while measuring, both
numbers are useful when reading). Angle kind: "45°". Pinned at B it stays at B. With Shift (§5) the
numbers appear as the dimension text instead and the chip is hidden (one set of numbers on screen).
`aria-hidden`; the polite live region speaks the final value on a pin ("12.5 millimetres, 45 degrees")
and on a placement, never on every move.

**Measure look:** line A to B 1px solid `--preview-new`, 8px end ticks perpendicular to the line,
constant on screen; blue means "preview, nothing stored", the same as the Pen. Radius: a 1px line from the
centre through the pick point, tick at the outline.

## 4. Mode label (same component and placement as 0066 §2)

Chip, `pointer-events: none`, `aria-hidden`, at most 22 characters, Title case: the kind alone while
measuring, **"Dimension, " plus the kind while Shift is held or in the placement phase**:
"Aligned", "Horizontal", "Vertical", "Angle", "Radius", "Diameter"; "Dimension, aligned" and so on
(longest "Dimension, horizontal", 21). Before the first pick: no label (customer feedback on PR #243). Placement and
quadrant flip as 0066 §2 (S = A once set, else the pointer). The word is the non-colour cue for
measure versus dimension. A visually hidden polite live region says the label on each change and
once when the tool opens.

## 5. Leaving a dimension: Shift (decided, Proposal)

**Shift held at the pick of the second anchor (release of the drag, or the second click) makes the
gesture a dimension; without it the gesture is a measurement.** For a curve pick the same pick
moment applies (the pick of the circle). Rationale: Shift is the Select family's "add, keep" key, it
needs no control and no state to explain, and a toggle would be the only mode switch in a tool with
no bar. Selection first (§2) creates on the placing click and needs no Shift: the click is the intent.

- Shift is read live (window-level key events, as Ctrl in the Pen) from the first pick on. Pressing
  or releasing it swaps the blue measure drawing and the readout chip for the green dimension
  preview and back **in the same frame**; the mode label follows.
- Shift never means axis lock here. Ctrl locks the A to B direction to the 15 and 22.5 degree stops
  (same table as the Pen and the Lock badge, 16px left and below the pointer). It affects the pick
  only, not the kind.
- Placement click commits; Escape cancels the gesture (cascade, §9); Shift may be released before the
  placing click.
- Touch and other pointers without a keyboard cannot leave a dimension in this slice (desktop first,
  mobile gets no code until a story needs it). Recorded in `docs/technical-debt.md` by the lead if
  accepted.

## 6. Snapping and hover cues

- **Snap marker:** the 0060 marker, unchanged (`--reference-mark` line and 12px cross, white casing,
  no text). Snaps are nodes, centres and midpoints of shapes (criterion 6) and guides' nodes. Each
  anchor picks its own marker; the marker shows before the press and during the drag.
- **Hover highlight of a pick source:** 4px `--segment-hover` band along the segment, arc or circle
  outline under the pointer (the Node tool's segment hover); a snap point shows the marker instead.
  Cursor stays `crosshair` (the marker is the state, 0066 §6 rule).
- Alt turns snapping off for the gesture as everywhere in Vector (0060 §10, §11); with it the
  outline of a curve near a node can be picked (§8).
- The placement of a dimension line (offset, text) is **never snapped**; only anchors are.

## 7. Dimension look, states, linked and unlinked

**Layout: exactly the 0066 §7 anatomy**, drawn by the same render code, text by the same label
component: two extension lines (start 4px off the point, 4px past the dimension line), a dimension
line, filled arrowheads 8px long and 6px wide, text 12px tabular centred and breaking the line
with the 2px `--dim-halo`, default offset 24px with the rank rule, radius leader and "R 25 mm", diameter
through the centre and "⌀ 50 mm", angle arc and "45°". Weight 1px, all sizes constant on screen.
Small spans: under 56px arrowheads flip outside, under 24px ticks only; **a Vector dimension is
never hidden** by the 16px rule of 0066 (the maker placed it on purpose). One colour: new alias
`--dim-guide` = `--guide` (`#166534`: 5.8:1 on `--canvas-bg`, 3.6:1 on `--pasteboard-bg`; the text
carries the halo). Lines are **solid**: the guide dash (2 on / 2 off) is for guide lines, not
dimensions (DIM-GREEN, DIM-LOOK). A reference and a Vector dimension of the same value differ only
in colour (criterion 7).

| State | Drawing |
|---|---|
| Linked (default) | as above |
| **Unlinked** (a measured node or object was deleted, criterion 12) | extension and dimension lines **dashed 4 on / 3 off** (0066 reference rhythm, not the 2 / 2 guide dash) and a **10px Lucide `Unlink` glyph, 1.5px stroke, `--dim-guide`, before the text**; text stays "12.5 mm", no parentheses (parentheses mean "reference" in 0066). Accessible name "12.5 millimetres, not linked". Colour is unchanged, dash and glyph carry the state |
| Hover (Select tool) | 6px `--accent-hover` band behind line and text, cursor `grab` over text or line |
| Selected | every part 1.5px and `--accent`, text in a 1px `--accent` pill (0066 §7); **no bounding box, no handles**; the measured objects get their normal hover highlight, not selection |
| Keyboard focus | none on the canvas; the Layers row takes it (§10) |
| Locked or hidden layer | follows the layer like any guide (criterion 15) |
| Unlinked and selected | both rules: accent parts, dashed, glyph in `--accent` |

**Select-tool behaviour of a selected dimension:** drag the text along the line (criterion 8) and across
(the offset), one history step "Move dimension"; drag a dimension line moves the offset. Arrow-key nudge,
resize, rotate and the group box do nothing for a lone dimension and show no notice. Moving or scaling
the **measured** objects moves their dimensions live in the same frame (criterion 11) as part of that
one gesture's step. Double-click does nothing (nothing is driven; no entry box exists here).
The Select bar shows what it shows for a selected guide (0046), no dimension-specific group; the Style
tab behaves as for a guide. Nothing new in the right panel.

**Convert to path (criterion 10):** with a dimension selected the **Guide button shows pressed**
(`aria-pressed`, `--toolbar-icon-active-bg`); pressing it converts the dimension to ordinary paths and
text outlines in one step, history name "Convert to path", the new objects are selected and the button
is unpressed. Pressing it again makes those paths guides ("Guide", plain guides, the link is not
rebuilt, criterion 10). Tooltip stays "Guide" (no hint). A selection that mixes dimensions and
ordinary objects follows the existing Guide rule (pressed only when every markable node is a guide).
The converted line style is the marking style of 0046 so nothing jumps on the canvas.

## 8. Alt: snapping versus Radius and Diameter (resolved)

Alt has two jobs in this tool. They are separated by phase, never by context guessing:

| Phase | Alt does |
|---|---|
| Picking an anchor or curve (press, drag, hover) | **Held**: inverts snapping (0060 §10, §11), as in every Vector tool. On a curve with a node near the pointer, Alt picks the outline instead of the node |
| Placement of a Radius or Diameter (curve pick done, placing click not yet made) | **Keydown, once per press**: switches Radius and Diameter. The label shows "Radius" or "Diameter" (and "Dimension, " prefixed), the preview swaps in the same frame |
| Any other kind in placement | nothing |

Why this has no clash: the placement of a radius dimension never snaps (§6), so inversion has no
effect in that phase, and a curve pick has no cycle. An Alt already held when the pick happened
does not cycle; releasing and pressing again does. Rules as 0066 §2: keydown only, `repeat`
ignored, no key-up needed (WebKitGTK), `preventDefault` so no menu takes focus, not with AltGr, not
with Ctrl, Cmd or Shift also down (Shift is the dimension key: Shift+Alt is therefore ignored, and
the user releases Shift or presses Alt alone), ignored in a field. The default is **Radius** for
circle, arc and corner (criterion 3; note 0066's review item 3 saw the opposite in the Sketch
tool, which is that tool's own fix). Linux window managers that take Alt+drag are the existing
Vector limitation; this tool adds no Alt drag.

## 9. Keyboard, accessibility, history

| Input | Effect |
|---|---|
| `M` / rail button | Measure tool; Tab lands on the Path-tools card (roving) |
| Escape | cascade, one step per press: cancel the placement or pinned measurement; else leave to Select (selection kept) |
| Shift | leave a dimension (§5) |
| Ctrl | lock A to B to 15 and 22.5 degree stops, with the Lock badge |
| Alt | invert snapping while picking; keydown cycles R and ⌀ while placing a radius (§8) |
| Enter | with an eligible selection, place at the default position (§2) |
| Delete | with a dimension selected (Select tool): delete |

- **Pointer-only gestures, keyboard routes:** creation by Enter on a selection (reachable through the
  Layers tab or canvas selection); selecting, reading and deleting a dimension through the Layers
  row. Dimensions get a row **"Dimension 12.5 mm"** (Lucide `Ruler` 16px), guide marking as other
  guides, with "not linked" appended when unlinked; the selection announcer says the same.
  Dimension texts on the canvas are `aria-hidden` labels, not focus stops (hundreds would flood Tab).
- Contrast: `--dim-guide` text and lines 5.8:1 on `--canvas-bg`; on the pasteboard 3.6:1 with the
  halo. Targets: text box at least 24px high, lines 8px tolerance, anchors 8px.
- Large documents: dimensions follow measured objects through the same incremental update as
  linked clones; over 150 dimensions on screen, arrowheads and ticks stay, nothing is thinned
  (they are deliberate marks); 200 dimensions must drag-follow at the frame rate of a 200-object
  move (acceptance test, not a UX claim).
- **History names:** "Dimension" (create, one step), "Move dimension" (text or offset drag),
  "Convert to path" (Guide off on a dimension), "Guide" (paths to guide); delete is the existing
  "Delete", including the unlink in the same step so one undo restores object, link and dimension.
  Moving or scaling measured objects is the move or transform step, with no extra step.
  A measurement writes nothing.
- Reduced motion: nothing animates; every cue appears and goes in the frame of its cause.

## 10. Tokens and design-system rows

New: **`--dim-guide`** = alias of `--guide` (`#166534`), light theme only as today. Reused, unchanged:
`--guide`, `--dim-halo`, `--preview-new`, `--accent`, `--accent-hover`, `--segment-hover`,
`--reference-mark` (snap marker), `--toolbar-bg`, `--toolbar-icon`, `--toolbar-icon-active-bg`.
When the build lands the lead (or I) adds the rows "Measure tool", "Measure readout", "Vector dimension
look" to `docs/design-system.md`, the Shortcuts row "Measure tool: `M`", and the rail budget line
(column A ends 580px).

## 11. Open points for the customer (each with a default)

- **MEAS-RAIL** (§1): where the tool lives. Default A: second button of the Path-tools card.
- **MEAS-KEEP** (§5): Shift leaves a dimension, no toggle. Default: Shift (the customer can veto
  at the demo; the alternative is a pressed state on the rail button, which costs a second
  button).
- Specification text to correct (PO): criterion 2 (no hint line, no bar), criterion 4 (Shift read at the
  second pick), criterion 6 (phase rule of §8).

## 12. Review checklist for the implemented UI

Rail card (two buttons, one Tab stop, tooltip with and without selection, "Esc, M"); M letter gating;
three input forms; Shift swapping blue measure and green dimension in one frame and the label text;
the eight placement regions; curve pick Radius default, Alt keydown cycle, held-Alt snapping
inversion, no cycle when Alt was already down; selection-first preview, pick source priority, Enter
placement; readout format at 3 decimals, minus sign, flip at edges; snap marker and hover band;
pinned measurement ends on all listed events; Escape cascade; linked live update during node drag,
scale and rotate; delete of a measured node gives dashed lines and the `Unlink` glyph, undo restores;
Guide button pressed, converts, round trip; Layers row and announcer text; zoom 25, 100, 400
percent for the tiers; 200 dimensions; light theme contrast on `--canvas-bg` and `--pasteboard-bg`.

## UI review (2026-11-01)

Reviewer: ux-engineer. Build of `c7a3bc97`, `vite preview` on 1440x1000 and 900x640, Chromium, light
theme, side by side with a 0066 Sketch radius dimension. Screenshots: `target/0070-browser/ux/`
(`00-sketch-dimension.png` Sketch reference, `11-all.png` all six kinds, `03-measuring.png`,
`04-shift-live.png`, `15-selected.png`, `18-unlinked.png`, `54-group-scaled.png`, `62-history.png`,
`32-narrow.png`). No page errors in any run.

**Verdict: APPROVE WITH CHANGES.** Everything the customer asked for works: the tool is reachable, the six
kinds can be left, they follow their geometry, unlink and relink, convert and undo. The changes below are
polish and consistency, none blocks the demo except where marked.

### Verified (works as specified)

- Rail: Measure is the second button of the "More tools" card, `aria-pressed`, one Tab stop, ArrowUp moves
  within the toolbar. Tooltip "Measure tool (M)", and "Measure tool (Esc, M)" while an object is selected.
  `M` selects the tool (also with the Select tool and nothing selected).
- Measurement: readout chip "29.581 mm, 26.565°" / "Δ 26.458, 13.229 mm", blue line with dots, nothing
  stored (no history step). No mode label before the first pick, then "Aligned".
- Shift swaps the blue drawing and chip for the green dimension and the label to "Dimension, aligned" in
  the same frame. Placement regions give "Dimension, horizontal / vertical", curve pick gives radius,
  Alt keydown switches to diameter, two segments give angle. Layers speech and rows match.
- Look: same arrowheads, 12px tabular text, halo, extension lines and offset as the Sketch dimension, in
  `--dim-guide` green, solid. Selected: accent lines and a text pill, no box and no handles.
- Linked: node drag (Node tool) changes the angle live (90° to 86.448°); group scale changes the aligned
  value (47.698 to 41.837 mm); move follows; all undo as one step. Deleting a measured node or object gives
  dashed lines and the `Unlink` glyph, undo relinks. Delete of a selected dimension and its undo work.
- Text drag along and across moves the text; history reads "Dimension", "Move dimension", "Delete",
  "Convert to path"; Guide button shows pressed for a dimension, converts to paths (5 labels left), undo
  restores (6).
- Snapping: marker on a node, Alt held picks the free point. 900x640: canvas, rails and panel stay usable.

### Required changes

1. **Layers row truncates the value** (`18`/`15` Layers shots, `[data-...]` rows): "Dimension ∅ 26.4…",
   "Dimension 47.698…" at the default panel width; "not linked" is never visible and the row has no
   `title`. Fix: drop the word "Dimension" from the visible text (the Ruler icon names the kind; keep the
   full "Dimension …" as the accessible name), so "⌀ 26.458 mm" and "90° · not linked" fit; add a `title`
   with the full name.
2. **Layers row icon is the generic guide glyph, not Lucide `Ruler` 16px** (§9). Use `Ruler`.
3. **A selected dimension offers controls that do nothing for it** (`15-selected`, `45-scaling`): the Style tab
   is titled "Dimension" and shows Stroke and Fill toggles; the Select bar shows an enabled "Linked clone"
   and, mixed with objects (`51-all-selected`), "Radius 0 mm" and the corner-link toggle count dimensions.
   §7 says the bar and tab behave as for a guide, no dimension group. Fix: for a selection that is only
   dimensions, hide Stroke and Fill in Style (as for a guide: nothing to edit) and disable "Linked clone"
   and "Stroke to path"; dimensions must not enable the Radius field in a mixed selection.
4. **First drag directly after selecting a dimension by click is swallowed** (`61-*`, reproduced 2 of 2 with a
   click followed by a press within about 150 ms on the text; later drags work). Likely the press is read as
   the start of a double-click, which does nothing by design. Confirm by waiting 500 ms: if the drag then
   works, accept and drop this item; if not, the first press after a selecting click must start the drag.
5. **Design-system rows are missing**: `docs/design-system.md` has `--dim-guide` but not the rows "Measure
   tool", "Measure readout", "Vector dimension look", the Shortcuts row "Measure tool: `M`" and the rail
   budget line (column A ends 580px), all promised in §10. Add them with the first PR commit.
6. **Shift preview with the first anchor only** (`04-shift-live`): the A dot is gone and the dimension is drawn
   as two short extension stubs with the text across the line while the pointer is still on B. Keep the A
   dot while Shift is held (same as the blue state) so the pick position stays visible. Polish, not blocking.

### Open points and notes (no change required)

- A vertical dimension placed close to its object (default 24px, or a user placement under about 36px)
  puts the 60px text against the extension line (`11-all`, "↕ 26.458 mm" touches the rectangle). The 0066
  rank rule has the same property; consider raising the default offset for vertical dimensions to half the
  text width plus 8px. Recorded, not required.
- The Sketch reference dimension in `00-sketch-dimension.png` is drawn in red (a driving or reference
  state of that tool); the Vector dimension intentionally uses guide green, as the customer asked.
- Not checked: dark theme (no dark theme exists yet), 200 dimensions (acceptance test, not a UX claim).
