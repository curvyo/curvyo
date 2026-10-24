# 0080 Text on path: UX notes

Owner: ux-engineer, 2026-10-24. For `specification.md` in this folder. Rows of
`docs/design-system.md` are named in quotes. Bar widths and line counts use
the measured values of `0084` (UX notes §6, rows "Text bar" and "Select bar
Text group"): pill padding 8px each side, 25px between groups (divider
included), 4px inside a group, 36px lines; inner width 308px at 800 x 600 and
about 788px at 1280 x 800 (panel open).

## Customer decisions 2026-10-24 (slice 1, before merge)

These replace what §0.2, §3, §5 and §7 say below where they differ.

1. **The position handle sits at the middle of the text** on the carrier, and Position is the
   arc length of the text's middle. Mirror and Mirror letters keep the text where it is; Ctrl
   snapping puts the middle on start, middle and end; a new text on a closed shape is centred at
   the top; the typed `R` turns about the middle. On an open carrier a new text still starts at
   the carrier's start.
2. **Position, Distance, Side, Line and Mirror leave row 2.** They appear only in the **entry box
   of the position handle**, opened by a double-click on the handle: the handle entry chip
   pattern (`0008`, `0052`, `0072`), the "Transform entry chip" surface, top left 16px right of
   and below the handle. One row: Position (136px, "mm" or "°"), Distance (136px, "mm"), the Side
   radios, the Line on path radios, Mirror. Position has the focus with its text selected; Tab
   cycles fields and buttons inside the box; the buttons preview on the canvas; Enter writes one
   step (the single change's own name, "Change text on path" for several); Escape or leaving the
   box closes it with no change; the handle keeps its dragging look while it is open. Release
   from path stays in row 2; the row-2 dot still says "Changed: On path".
3. **Bug:** after a double-click into edit mode, Escape twice and V (Flip vertically), moving the
   pointer slid the text along its carrier. Fixed; no drag or press is left over after edit mode.

## 0. Proposed changes to the specification

All are Proposals the customer can veto at the demo.

1. **Row 2 order (amends `0084` criterion 7 and the design-system row "Text
   bar").** Row 2 becomes: Writing mode, Glyph orientation, **Glyphs, then
   Taper**, then the On path groups. Taper is not built yet, so this moves no
   built control; it follows the rule that a new control is appended. It is
   what makes a text on path fit 5 lines at 800 x 600 (§7); with Taper before
   Glyphs it is 6.
2. **On path options are three groups (criterion 17 and `0084` criterion 7,
   which names only Side, Line, Release):** "Placement" (Side, Line, Mirror),
   "On path values" (Position, Distance), Release from path. All on row 2.
3. **Preview while picking (criterion 2).** While the pointer is over a
   carrier in pick mode, the text is drawn on it at its default placement
   (criterion 5) in the preview look of `0009`; the straight text stays drawn
   where it is. The maker sees side, start and direction before the click.
4. **Pick mode returns to where it started (criterion 2).** Pressed from edit
   mode, edit mode commits first ("Edit text", if anything changed); after the
   put, Escape or a right click, edit mode resumes on the same text with the
   caret at the same character index.
5. **On path on a text that is on a path already (criterion 1)** starts pick
   mode too; the click moves the text to the new carrier (Side, Line,
   Distance, Mirror letters kept, Position to the default of criterion 5), one
   step "Put text on path". The old carrier stays a guide (criterion 4a).
6. **Enabled with exactly one text (criteria 1, 2).** With two or more texts
   among the subjects the button stays in place, `aria-disabled` at 40 %,
   tooltip line 2 names the state: "3 texts selected".
7. **Keyboard nudge (criterion 21).** Left and Right move Position against
   and in **reading direction** (not carrier direction: on the bottom of a
   clockwise circle the carrier runs right to left on screen and Right would
   move the text left). Up and Down change Distance by 1 mm (Shift 10 mm), Up
   towards the top of the letters. One step per held run, as "Nudge".
8. **Hit tie (criterion 28).** Where a letter box of a text and the line of
   its own carrier are both within the hit tolerance, the text is hit. The
   carrier is reached on its line away from the letters, by marquee or in the
   Layers tab.
9. **Ctrl snap of the position handle (criterion 15)**: §3.3.
10. **Carrier line while the text is selected (criteria 7, 15, 25)**: §4.

## 1. On path button (criterion 1)

- **Text bar row 1**, between the Style group and the expand button, as
  `0084` criterion 6 reserves (28 x 28, divider before it). **Select bar**: in
  row 1 of the Text group at the same place (Style, On path, expand). It is in
  the tree whenever the Text bar's row 1 is, so nothing moves when a text is
  selected or put on a path.
- 28 x 28 icon button, the Glyphs button's look (no outline, `rounded-md`,
  hover `--accent-hover`, focus ring 2px `--accent`, 1px offset). **Glyph**
  (custom, row "Command glyphs" at 16px, 1.5px stroke, `currentColor`): an arc
  `M2 13.5 Q8 4.5 14 13.5` with a "T" standing on its apex (stem 8,9 to 8,3.5;
  bar 5.5,3.5 to 10.5,3.5). No Lucide icon says "text on a curve"; `Spline`
  reads as the Node tool and `SplinePointer` as a cursor.
- `aria-pressed` true only while pick mode runs (the Italic toggle's pressed
  look). It is a command, not a state of the text: a text on a path does not
  show it pressed (row 2 and the dot of the expand button show that, §2).
- Tooltip (Radix, 400 ms, below): "On path"; line 2 only for the disabled
  state of §0.6. Accessible name "On path". No shortcut (as Object to path and
  Linked clone); keyboard route: select the text and the carrier, Tab to the
  button, Enter (criterion 3).

## 2. Pick mode (criteria 2, 3, 7)

- **Start.** A press with exactly one text as subject and no carrier selected
  with it. With the Select tool and one text plus carriers selected, the press
  puts the text on them at once (criterion 3; the carriers are the selected
  objects of a carrier kind, anything else in the selection is ignored).
- **Cursor.** New "Pick carrier" cursor, 24px, the white halo under black of
  the Pen cursors: the Select arrow with the On path glyph at 10px at its
  lower right; hotspot at the arrow tip; `crosshair` where custom images are
  ignored. The same cursor over everything (no `not-allowed`): the hover
  shows what a click would take.
- **Hover.** Only the carrier under the pointer: its outline (for a compound
  path only the outline under the pointer) drawn in `--hover-box` at 2px with
  its casing, plus the preview of §0.3. Hit rules of the Select tool: within
  8px of the outline ("Object hit-test tolerance"), inside a filled closed
  shape, a guide by its line only; the deepest shape under the pointer, inside
  groups without entering them; carriers on locked layers included
  (criterion 7). Texts, images, groups as a whole and the subject text itself
  show nothing.
- **Click** on a carrier: one step "Put text on path", pick mode ends, the
  text (alone) is selected with its handles (§3), the carrier is drawn as a
  guide in the same frame. A polite live region says "Put on path"; no
  visible notice (spec). Click on nothing: nothing, pick mode stays.
- **End without change:** Escape (its own step of the "Escape cascade", before
  the selection clear), a right click, a second press of On path with an
  empty pick set, a tool switch, the subject text removed (Undo, a peer).
  Ctrl+Z ends pick mode, then undoes as usual. Space-drag pan and wheel zoom
  work and do not end it. Nothing in pick mode writes history.
- **Refusal:** a locked text is refused at the press with the Pen's lock
  notice (criterion 7), pick mode does not start.

## 3. Canvas handles (criteria 15, 16, 20, 21)

### 3.1 Position handle

- The "Parameter handle" (10px circle, ring, centre dot, idle / hover /
  dragged grounds, 12px hit radius, `pointer` cursor) on the carrier at the
  text's middle (customer 2026-10-24). A double-click opens its entry box
  (customer decision 2 above). With Distance not 0, a "Parameter handle guide" (1px dashed)
  runs along the normal from the handle to the line on path, so the handle
  stays on the carrier and the gap is visible.
- Shown with exactly one text on path selected (Select or Text tool, also in
  edit mode), as all parameter handles; not during a move, scale or rotate
  drag; drawn above the selection box handles. No size threshold `T`: it is
  the only parameter handle of a text and sits on the carrier, not inside the
  box.
- The carrier's own handles (radius, arc, curve) appear only when the carrier
  is selected; text and carrier selected together are two objects, so neither
  shows parameter handles ("Parameter handle layout": none for two or more).

### 3.2 Drag

- The text's middle follows the carrier point nearest the pointer;
  continuous across the start of a closed carrier. Preview in the `0009`
  look, the carrier line of §4 stays. Readout chip ("Live transform
  readout"): "42.3 mm", on an ellipse carrier "90.0°", plus " · start",
  " · middle", " · end" at a snap point, plus " · Below, top edge" while the
  drag has crossed the carrier (criterion 16, the state after release).
- Crossing: more than 8 screen px beyond the carrier on the other side swaps
  Side and Line in the preview at once (no animation); back within 8px swaps
  back. One step on release ("Move text on path"; with a crossing "Mirror
  text on path"). Escape cancels; the release then writes nothing.

### 3.3 Snapping

- Hold **Ctrl** (the convention of the arc handles and rotate): open carrier
  and arc: start, middle, end of its length; closed carrier: the start, the
  quarter lengths from it and the default point (criterion 5); ellipse: the
  15° and 22.5° stops of the arc handles. No magnetic snap without Ctrl.

### 3.4 Rotation on an ellipse (criterion 20)

- The rotate handles of the selection box, the "Transform rotate handle"
  look; the pivot is the ellipse's centre, shown during the drag with the
  "Transform center move handle" glyph there (not draggable). Readout "90.0°"
  (the new Position). Ctrl snaps to 15° and 22.5°. `R` opens the rotate entry.

### 3.5 Other carriers (criterion 21)

- Box: corner resize handles only (Size, reference point fixed); no edge,
  rotate or skew handles. A body drag slides the text as the position handle
  does (same readout); it never takes the text off the carrier.

## 4. Carrier as guide, selection

- A carrier is drawn as every guide (row "Guide (`0046`)": `--guide`, 1px,
  2 on / 2 off). Nothing else marks it at rest; the picture stays the
  production picture.
- **Carrier line (§0.10):** while one or more texts on path are selected, each
  one's carrier outline is drawn 1px solid in `--member-box`, over the guide
  dashes. This is the only place the maker sees what the text is attached
  to, and the only look of a carrier that is not a guide (locked layer,
  criterion 7) or a kept outline of a deleted carrier (criterion 25).
- **Click** on a letter: the text. On the carrier's line away from the
  letters: the carrier (a guide, so not inside it). Tie: §0.8. Shift+click
  adds as usual, so text and carrier can be selected together (criterion 22).
  Double-click on a letter: edit mode (caret along the carrier, turned with
  its letter, `0074` caret look, at least 12px). Double-click on the carrier:
  what it does for its kind today.
- **Edit mode:** no line box (it is axis-parallel and means nothing along a
  curve); the selection box of criterion 28 dashed, the carrier line, the
  position handle. Text selection highlight per letter, turned with it.

## 5. Row 2: On path options (criteria 17 to 19, 23)

Superseded by customer decision 2 above: row 2 keeps only Release from path;
Placement and On path values are the controls of the handle's entry box,
with the glyphs and names below. The table is the build of 2026-10-24
morning, kept for the glyphs.

Shown when a subject text is on a path, after Taper (§0.1). Every control
acts on the texts on path among the subjects only; Mixed as everywhere.

| Group | Controls | Width |
|---|---|---|
| Placement | `radiogroup` "Side": Above, Below; `radiogroup` "Line on path": Baseline, Top edge (28 x 28, 4px apart, the Alignment group's look and arrow keys); Mirror (28 x 28 command button) | 60 + 4 + 60 + 4 + 28 = 156 |
| On path values | "Position" (bar number field, 120px, "mm", or "°" on an ellipse; steps 1, Shift 10, Ctrl 0.1), "Distance" (120px, "mm", may be negative, same steps) | 244 |
| Release | Release from path, 28 x 28 command button | 28 |

- **Glyphs** (custom, 16px, 1.5px): Above: a line at y 10 with an up arrow
  above it; Below: the same with a down arrow under it. Baseline: a line at
  y 13 with an "H" standing on it; Top edge: a line at y 3 with an "H"
  hanging from it. Mirror: a line at y 8 crossed by a vertical arrow with two
  heads (not the Flip glyph: Flip mirrors the letters, criterion 19). Release:
  the dashed arc of the guide at the bottom, a straight solid line above it
  with a "T" on it.
- Tooltips name the control only: "Above", "Below", "Baseline", "Top edge",
  "Mirror", "Release from path". Field labels inside the left edge.
- **Changed dot** of the expand button (`0084` criterion 13): on while a
  subject text is on a path and row 2 is closed; description "Changed: On
  path". Row 2 does not open by itself (`0084`, out of scope).
- **Vertical text on a path (criterion 48):** Writing mode and Glyph
  orientation greyed (`aria-disabled`, 40 %), tooltip line 2 "Text on a path
  runs along the path" (a state, kept from the spec).
- **Release from path:** the text stays selected, straight, same Size; the
  carrier stays a guide; row 2 drops the On path groups in the same frame.
- **Flip H / V** (criterion 19) on a text on path: the existing Flip buttons
  and keys, readout and live region "Flipped horizontally"; nothing new in the
  bar.
- **Object to path:** the existing Select bar button; the group is selected
  after it, the carrier stays a guide. Nothing new.

## 6. Empty, error and large-document states

- Text longer than the carrier: no tag, no notice (Frage 37).
- Deleted carrier: the carrier line of §4 shows the kept outline; Layers
  tooltip per criterion 25. Notice text per criterion 25.
- Empty string: as `0074` (removed on leaving edit mode); the carrier stays a
  guide.
- Large documents: pick hover uses the Select tool's spatial index; the
  preview of §0.3 is laid out once per hovered carrier and cached until the
  pointer leaves it; only the hovered carrier is highlighted, never all
  carriers. Carrier lines are drawn for selected texts only.
- Undo: every step of the spec, one per gesture; pick mode, hover and the
  preview write nothing.

## 7. Fit tables (row 1 unchanged from `0084`)

Customer decision 2 above shortens row 2 to Release from path (28px): the
tables below are the counts before it; the On path lines shrink to one
28px group.

### Text bar, 800 x 600 (inner 308px)

| Line | Controls | Width |
|---|---|---|
| 1 | Font, Size, Alignment | 301 |
| 2 | Style, On path, expand | 302 |
| 3 (open) | Writing mode, Glyph orientation, Glyphs | 262 |
| 4 (open) | Taper, Placement | 301 (until Taper ships: 156) |
| 5 (open, on path) | On path values, Release | 297 |

Closed 2 lines (72px); open 4 (144px), with a text on path 5 (180px), as
`0084` §6 foresaw. Without §0.1 (Taper before Glyphs) it is 6 lines.

### Text bar, 1280 x 800 (row 1 628px; row 2 not wider)

| Line | Controls | Width |
|---|---|---|
| 1 | Font, Size, Alignment, Style, On path, expand | 628 |
| 2 (open) | Writing mode, Glyph orientation, Glyphs, Taper, Placement | 588 |
| 3 (open, on path) | On path values, Release | 297 |

Closed 1 line, open 2, with a text on path 3 (108px).

### Select bar, one text on path selected

800 x 600: closed 5 lines (180px, unchanged), open 8 lines (288px: row 2 is
lines 3 to 5 of the Text bar table). 1280 x 800: closed 2 lines, open 4
(144px; row 2 is 910px and wraps once). Computed; the build PR measures them
and writes the counts into the rows "Text bar" and "Select bar Text group".

## 8. Design-system changes for the build PR

- New rows: "On path button", "Pick carrier cursor", "Pick mode" (hover,
  preview, ends), "Position handle" (a use of "Parameter handle"), "Carrier
  line", "On path options".
- Rows "Text bar" and "Select bar Text group": On path built, row 2 order of
  §0.1, measured counts. "Escape cascade": pick mode ends before the
  selection clears. "Keyboard shortcuts": nudge of a text on path (§0.7), Ctrl
  snap of the position handle.

## 9. Slice 2 sketch: several paths (criteria 37 to 42)

- **Pick set:** Shift+click adds a carrier and keeps pick mode; Shift+click
  on a picked one removes it. Each picked carrier: its outline 2px `--accent`
  with casing (selected look) and the preview of §0.3 on it; the first picked
  (the source) carries a 16px badge "1" in the "Modifier badge" look at its
  start, the others none (the order beyond the source does not matter).
  Enter or a second press of On path puts on the set; Escape drops the whole
  set. Notice "Put text on 3 paths." in the canvas notice slot.
- **Per-clone handles:** each text clone has its own position handle, carrier
  line, row 2 values, nudge and rotation, shown when that clone alone is
  selected. With several selected: no position handles (two or more objects),
  row 2 shows Mixed and acts on each; nudge moves each along its own carrier.
- **Marks and editing:** a selected clone shows the "Linked clone marks" on
  the source and the other clones. Edit mode starts on the clicked clone; the
  others change in the same frame, without caret. Style tab: the `0049` clone
  block and the `0057` Style row, no Placement row.
- **Fit:** nothing new in the bars; the Linked clone button of the Select bar
  stays for paths (out of scope for texts).
