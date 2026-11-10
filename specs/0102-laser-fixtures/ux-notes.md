# 0102 Laser fixtures: UX notes

ux-engineer, 2026-11-10, revision after the review of PR #313. Customer, translated: "The UI is not finished yet. Use the rectangle as the model: take its handles including all its double-click entry boxes. For holder and clamp the input fields in the property bar go away. For the stop, do the analogous thing that fits. The grid can stay as it is. There is also a fixture for a screw/bolt with the dimensions of the thread and of the screw head. Assume they are round."

Checked against `specification.md` (revised the same day: criteria 1 to 40, Fragen 171 to 175), `specs/0101-laser-operations/ux-notes.md` (the operations model), `specs/0093-laser-workbench-shell/ux-notes.md` (the eye, superseded where this file says so), `specs/0106-move-versus-edit/ux-notes.md` (the Fixtures row of Layers), `specs/0060-snapping/ux-notes.md` (the snap marker), `docs/design-system.md` (the rectangle's rows, section 0, which cite `0008`, `0019`, `0072` and `0073`), the chip code `frontend/src/components/TransformEntryChip.tsx`, `MoveEntryChip.tsx` and `HandleHintChip.tsx`, and the current `FixtureSelectBar.tsx` (X, Y and Angle fields, to be removed). Customer rules applied: no operating tips or hint labels (2026-10-16), no notice after a success (2026-10-31), hidden not disabled, no popups in the panel, no cut or machine settings in a row (2026-11-10), symbols plus text, in-place rename by double-click, the armed trash, arrows plus drag for order, no invented colours, and the 2026-11-10 rules above. Items marked (Proposal) are vetoable at the demo; section 14 lists them.

Terminology: "fixture" and the five kinds Grid, Holder, Clamp, Stop, Screw. "Ink" means `--toolbar-icon`. "Chip" means the rectangle's entry chip (`Transform entry chip`, `Move entry chip`).

## 0. The model: the rectangle as built

The selected rectangle, from `docs/design-system.md` (rows "Bounding-box selection outline", "Transform resize handle", "Transform rotate handle", "Transform handle layout", "Transform entry chip", "Move entry chip", "Parameter handle", "Parameter handle layout", "Live transform readout", "Select bar layout", keyboard rows "Type a size", "Type an angle", "Type a move"):

| Part | What the rectangle has |
|---|---|
| Box | 1px dashed `--accent` box (4 on, 3 off), in the object's own rotated frame |
| Resize | 8 squircles, 8 x 8px: 4 corners and 4 edge midpoints (edge handles from a box side of 24px). Hit radius 16px. A drag keeps the opposite side or corner fixed. Shift: from the centre. Ctrl: keeps proportions |
| Rotate | 4 arc glyphs, 12px, 32px outside the corners, always. Shift adds 4 at the side midpoints. 22.5-degree snap |
| Skew | 4 at the side midpoints, 16px outside, from a box side of 24px. Converts a primitive to a path (not used for fixtures, D22) |
| Centre move | at the centre, from a box side of 48px. A press inside the selected box that hits no handle moves the object |
| Parameter knobs | 4 corner radius knobs, 10px round with a centre dot (`Parameter handle`), from a box side of 72px; dragged along the corner's diagonal |
| Entry boxes (double-click) | Resize handle: Size chip, fields "W" and "H" (chain glyph between them when linked). Rotate: Angle chip, one 80px field, "°". Skew: skew chip. Centre: Move chip, "X" and "Y", Relative or Absolute, the nine-point Reference. Corner knob: "r" chip, "All four corners" or "This corner only" |
| Keys | S: size chip, R: angle chip, K: skew chip, M: move chip. The same chips as the double-click, placed as the handle's chip would be |
| Chip rules | Enter commits once, Escape cancels, a press elsewhere cancels and writes nothing. Tab moves between fields. Placed outward of the handle, 10px clear of it, clamped inside the canvas, upright. Not a number: 2px `--field-invalid` border and "Enter a number" under the fields. A double-click on a resize handle keeps the drag's fixed point; S scales about the box centre |
| Select bar | Settings group ("Scale stroke width", "Scale corner radius"), Flip, Guide, Align, Distribute, Grid, the rectangle group (Radius 96px field, Link corners, Remove rounding), Object to path, Linked clone. **No X, Y, Width, Height or Angle field.** Sizes and positions are the chips |
| Readouts | Live readout at the pointer, 12px: "W × H mm" for resize, "37.4°" for rotate, "r 3.5 mm" for a corner, "r 12.0 mm max" at a limit |
| Hint chip | A 600ms hint after resting on a handle ("Double-click: type a size"). Not used for fixtures (customer rule 2026-10-16, D23) |

## 1. Principle

A fixture is part of the bed, not of the drawing. Its outline and its handles are its controls. Active decides whether it lies on the bed; the eye only decides whether it is drawn. Kind is shape, never colour: the grid is lines, a holder is a board, a clamp is hatched, a stop is a solid bar, a screw is two rings. All fixture lines are ink. The one accent means selection and fit, as everywhere in the app.

Holder, clamp, stop and screw are sized and placed on the canvas with the rectangle's handles and entry boxes (section 0, section 6.2). The Document tab's column holds the grid's fields, the slots and the add group; it holds no size or position field of a holder, a clamp, a stop or a screw. Nothing opens a dialog or a popup.

## 2. Tokens (no new colour)

| Use | Token | Measure |
|---|---|---|
| Fixture ink: holder, slot, clamp outlines, clamp hatch, stop bar, screw rings, every fixture symbol | `--toolbar-icon` | `#3A3A3F`, 9.25:1 on `--canvas-bg` |
| Minor grid lines, 1 device px | `--grid-minor` | ink at 20 %, informational, as the page grid |
| Major grid lines, 2 device px | `--lane-line` | ink at 60 %, 3.45:1 on `--canvas-bg` |
| Tape ground, ticks, numbers, inner edge | `--ruler-bg`, `--ruler-tick`, `--ruler-tick-minor`, `--ruler-label`, `--ruler-edge` | the rulers' own look (the tape is a ruler on the bed) |
| Holder board ground | `--toolbar-bg` | light tint; the outline carries the edge |
| Slot interior, so a hole shows the bed | `--canvas-bg` | |
| Casing under every fixture line | `--selection-casing` | 1 px each side ("Casing over artwork") |
| Box, squircles, rotate glyphs, centre move, selected slot outline, fit highlight | `--accent`, `--accent-hover` | the one accent, as the rectangle |
| Knobs (holder: none; stop thickness; screw head and thread) | `--shape-handle-fill`, `--shape-handle-stroke`, `--accent` while dragged | the rectangle's Parameter handle, unchanged |
| Radius guide of a knob drag | `--shape-handle-guide` | as the rectangle |
| Drag ghost | `--drag-ghost` | |
| Tag border, Drop badge border | `--toolbar-icon` at 25 % | as the Drop badge of `0099` |
| Snap marker | the marker of `0060` | unchanged |

Dropped from the concept's sketch: `--fixture-ink` `#7A6F5F`, `--fixture-grid`, `--fixture-major`, `--fixture-tape`, `--fixture-board`. They are a warm brown that no other part of the app uses, which is the invented colour the customer rejected for `0101`. Dark theme: the tokens swap with the theme; no fixture value is defined here.

## 3. The Fixtures block (Document tab)

### 3.1 Place

- Document tab of a laser page, top to bottom: Pages, Unit, Machine block (with the profile rows of section 9), Place page (`0100`), **Fixtures block**, Background.
- Layers tree: the Fixtures row at the bottom (section 8).
- No Fixtures section in the Operations tab in this slice. Fixture jobs (`0103`) are what will put fixtures there. The Operations tab shows no fixture (`0101` criterion 10).

### 3.2 Heading and add group (criteria 1, 2)

- Heading row, 24px: "Fixtures", 14px `--toolbar-icon`, no button. The five add buttons replace the Plus of `0093`.
- Add group: `role="toolbar"` named "Add fixture", five buttons 4px apart, each **44 x 40px** (48px high on a coarse pointer). Five buttons of 44px with four gaps of 4px make 236px, the width of a row of the block (Proposal D28; four of 56px would not leave room for the fifth). Inside each: the kind symbol, 16px, above the word, 11px `--toolbar-icon`, centred. Words: Grid, Holder, Clamp, Stop, Screw. Accessible name and tooltip: "Add Grid", "Add Holder", "Add Clamp", "Add Stop", "Add Screw".
- Keys: Left and Right move, Enter or Space presses, one Tab stop. Focus stays on the pressed button, so a second press adds the next one (as `0101` section 6.1).
- A press adds one fixture named "<Kind> <N>" (lowest free N for that kind on the page), Active on, selected on the canvas, its row expanded if it has block fields (section 4.2; a screw or clamp has none, its row is not expanded). One undo step "Add <kind>", lower case kind ("Add screw"). No success notice.
- **Grid is hidden from the group while the page has one** (Proposal D6). The grid row is then the reason, so no refusal sentence is needed.
- Empty page: the heading and the five buttons, nothing else (criterion 1).

### 3.3 Rows (criteria 1, 3, 5, 6, 32)

List: `role="list"` named "Fixtures", creation order (no order control in this slice). One `listitem` per fixture. Rows are 40px high (48px coarse), two lines like an operation row of `0101` (Proposal D5: `0098` says 36px; this amends it).

Line 1, left to right, on a 24px line:

| Slot | Width | Content |
|---|---|---|
| Active | 16px | `.cb` check box (1.5px border, `role="checkbox"`, `aria-checked`). Checked: on the bed. Empty: not on the bed. |
| Gap | 8px | |
| Glyph | 16px | kind symbol (section 4.0), `aria-hidden` |
| Gap | 4px | |
| Name | flexible | 13px `--toolbar-icon`, one line, ellipsis; tooltip with the whole name; double-click or F2 renames |
| Show | 24px | `Eye` / `EyeOff`, `aria-pressed` (the eye of `0093`; one undo step, criterion 6) |
| Trash | 24px | `Trash2` 16px, `--panel-muted-fg`; armed look in section 3.5 |

Line 2 (11px `--panel-muted-fg`): the kind word; for a holder " · 2 slots" (the count); " · not on the bed" when Active is off. Examples: "Grid", "Holder · 2 slots", "Clamp · not on the bed", "Stop", "Screw".

- Active off: the name goes to `--panel-muted-fg`, the check box is empty, line 2 says "not on the bed". Muted ink plus a word, never opacity alone.
- Row states: hover `--editor-accent-hover`; selected (this fixture is the canvas selection) `--value-fill` ground and a 3px `--accent` bar at the left edge; focus 2px `--editor-accent` ring inside the row.
- A press on the glyph or the name selects the fixture on the canvas and expands its row if it has block fields (section 4). Press on the check box toggles Active. Press on the eye toggles the view. Press on the trash arms it.
- **Unknown fixture** (criterion 32): glyph `CircleHelp` 16px `--panel-muted-fg`, name kept, line 2 "Unknown fixture". No check box and no eye (they are not in the tree). The trash stays, the only way to remove it. It can be renamed.
- Windowing: rows are fixed height; at most 60 row elements in the DOM; the first rows paint at once (criterion 35 c).

### 3.4 Rename (criterion 3)

Double-click the name (or F2 on the selected row). The row's line 1 becomes a 13px text field: white ground, 1px `--editor-accent` border, 4px radius, from the glyph's place to the right edge. The check box, eye and trash leave the tree while the field is open (hidden, not disabled), so the field has the full width. Enter commits, leaving the field commits, Escape cancels. Up to 40 characters; names need not be unique (Proposal). Undo "Rename fixture". The field's name is "Name of <name>".

### 3.5 Delete: the armed trash (criterion 4, `0081`)

The same look and timings as the operations' trash of `0101` section 4.5: idle 24 x 24px `Trash2`; the first press arms (the row takes `--destructive-soft` on both lines, the trash becomes a 22 x 22px filled `--destructive` square with a white `Trash2`); a second press at least 400 ms later removes the fixture (a holder with its slots). Disarm after 4 s, on Escape, on a press elsewhere, when focus leaves, when the list changes. One armed trash in the app at a time. Delete on the focused row arms it; Delete or Enter on the armed row removes it.

Status region (sr-only, polite): "Press again to delete Holder 1. Its 2 slots go with it." on arming; "Holder 1 removed." on removal. No visible notice on removal; the undo label is the record. Undo "Remove fixture" restores the fixture with its slots, Active state, size and position.

Slot rows have the same trash (section 4.2). This supersedes `0093`, whose row removed at one press (Proposal D9).

## 4. Kinds, fields and entry boxes (criteria 15, 18 to 23, 37 to 39)

### 4.0 Kind symbols (16 x 16px grid, 1.5px stroke, `currentColor` ink)

| Kind | Symbol |
|---|---|
| Grid | `#`: two vertical and two horizontal lines, inset 2px |
| Holder | rectangle outline inset 2px with two small circles inside |
| Clamp | two brackets, `[` and `]`, with a bar between them |
| Stop | solid L, 3px thick |
| Screw | a circle outline inset 2px (the head) with a smaller concentric circle inside (the thread) |
| Unknown | `CircleHelp` 16px, `--panel-muted-fg` |
| Slot, rectangle | a small rectangle outline; slot, circle: a small circle outline |

The symbol is the row glyph (section 3.3) and the tag glyph (section 6.3). None of them reuses an operation symbol of `0101`, so a Clamp never reads as an Engrave. The screw's two rings tell it apart from a slot, which is one ring around a `--canvas-bg` hole.

### 4.1 Field look, commit and range

- **Block fields** (grid, slot): bar number field (`docs/design-system.md`): 28px high, white ground, 14px tabular text, label inside the left edge (12px `--panel-muted-fg`), unit inside the right edge (12px). Widths: 244px for one field, two fields 118px each with a 4px gap.
- **Entry chips** (sections 0 and 4.3): the rectangle's Transform entry chip: fields 100px wide, 4px apart, 28px high, visible labels inside the left edge (12px), unit inside the right edge. Open field 2px `--editor-accent` border; other field 1px `--toolbar-icon` at 60 %.
- Enter or blur commits (a chip: Enter commits, Escape cancels); Escape reverts; one undo step per commit.
- Up and Down change a block field by one unit (Shift: 10), committed on key-up, as the number fields of `0101`.
- **Range.** A value outside its range is set to the nearest limit with an Action notice 4px under the field or chip (status, 3 s, at most 70 characters). The pattern is "<Field> is <min> to <max> <unit>. Set to <value>." Where a limit depends on another value (section 4.3, D24), the limit shown is the current one.
- Drags stop at the limit and show no notice (the rectangle's limit readout, section 0).

### 4.2 Block fields by kind (top to bottom)

**Grid** (criterion 15, unchanged from the previous revision):
- **Spacing**: mm, 1 to 100, default 10 (244px).
- **Major every**: lines, 2 to 20, default 5 (244px, "lines" inside the right edge).
- **Tapes on**: a `role="group"` named "Tapes on" with four toggle buttons, 28px high, 58px wide, 4px apart: "Bottom", "Left", "Top", "Right", each `aria-pressed` (Proposal D20). Default: all four edges on (Frage 174 C). The numbers count from the origin corner of the 0090 grid (`grid_origin`, Frage 174 B). Any edge can be switched on or off; each press is one undo step.
- **Origin X**, **Origin Y**: mm, default 0 and 0, a pair of 118px fields (criterion 15).
- No Line width field (criterion 16 says the stored width is not shown in this slice; this removes the `0093` field, Proposal D8).
- No Select bar fields: the origin is typed here (section 6.4).

**Holder** (criteria 18, 19, 20):
- **No width, no height.** Its size is set on the canvas (section 4.3). The Fixtures block shows no size field for a holder.
- **Slots** heading row, 24px: "Slots" 12px `--panel-muted-fg` at the left, the count "2 of 50" at the right. The count is the reason the add button is gone at 50.
- Slot list: rows of 28px, one per slot in creation order: shape symbol 16px, name 13px, the text "Outside the board" 11px `--panel-muted-fg` when the slot lies outside (criterion 20; a text, not a colour), and the armed trash (section 3.5, name "Delete <slot name>"). Pressing a slot row selects the slot (section 6.2) and expands its fields. The slot list is windowed like the rows and sits in the tab's one scroll area.
- **Add slot**: a text button, full width, 28px, 1px `--toolbar-icon` at 60 % outline, 14px label. Creates "Slot <N>", a 20 x 20mm rectangle centred on the board, selected and expanded. One undo step "Add slot". At 50 slots the button is removed from the tree (Proposal D6).
- Expanded slot fields (under the selected slot row):
  - **Name**: text, 244px, "Name" inside the left edge, up to 40 characters.
  - **Shape**: `role="radiogroup"` named "Shape", two segments "Rectangle" and "Circle", 28px high, 118px each. Switching keeps the size: to a circle the diameter is the smaller of W and H; to a rectangle W and H are the diameter. One undo step "Set slot shape".
  - **Centre X**, **Centre Y**: mm from the board's corner, a pair of 118px fields.
  - **Width** and **Height** (rectangle) or **Diameter** (circle): mm, 0.5 to 1,000 (criterion 20), a pair of 118px fields or one 244px field.
  - Each edit is one undo step: "Rename slot", "Set slot position", "Set slot size".
- With no slot the list is empty under "Slots 0 of 50". No text.

**Clamp** (criterion 22): **no field**. Its size is set on the canvas (section 4.3). The block shows the row only.

**Stop** (criterion 23): **no field**. Its legs and thickness are set on the canvas (section 4.3). The block shows the row only.

**Screw** (criteria 37 to 39): **no field**. Its head and thread diameters are set on the canvas (section 4.3). The block shows the row only.

Undo labels for block edits: "Set grid spacing", "Set grid major", "Set tape edge", "Set grid origin", "Add slot", "Remove slot", "Rename slot", "Set slot shape", "Set slot position", "Set slot size". Names as in section 12.

### 4.3 Sizes and positions on the canvas: handles and their entry boxes

Each handle is the rectangle's, with the rectangle's double-click entry box. Holder and clamp use the rectangle's set minus skew and corner knobs (D22). The stop and the screw have their own set, the analogous one that fits (D23, D25). A keyboard key opens the entry box of the whole object (section 10).

| Kind | Handle | Drag | Double-click entry (chip) | Key chip (S) |
|---|---|---|---|---|
| Holder | 8 squircles: corners and edge midpoints | resizes W and H; the opposite side or corner stays fixed; Shift from the centre; Ctrl keeps proportions | a corner squircle: "W" and "H" (both fields, Tab between); an edge squircle: its one axis, "H" for n and s, "W" for e and w (as the rectangle) | "W" and "H" about the centre |
| Clamp | 8 squircles, as the holder | as the holder | "W" and "H" | "W" and "H" |
| Stop | Leg 1 squircle at the free end of leg 1, on its centreline | changes the length of leg 1; the corner stays fixed; stops at the thickness | "Leg 1", one field | "Leg 1", "Leg 2" and "Thickness" in one chip (row 1: legs, row 2, 6px below: thickness) |
| Stop | Leg 2 squircle at the free end of leg 2 | as leg 1 | "Leg 2", one field | as above |
| Stop | Thickness knob on the top edge of leg 1, at its midpoint (D23) | moves perpendicular to leg 1; the thickness is the distance from the bottom edge of leg 1; stops at the limit of section 4.4 | "Thickness", one field | as above |
| Screw | Head knob on the head circle at 0 degrees (east) | head diameter, through the centre; stops at the thread | "Head diameter", one field | "Head" and "Thread" (Tab between) |
| Screw | Thread knob on the thread circle at 180 degrees (west) | thread diameter, through the centre; stops at the head | "Thread diameter", one field | as above |

- The S chip of a stop and of a screw is one chip with all its fields; the other kinds' S chips are the same as their double-click chips.
- Undo: every commit of these handles and chips is one step "Resize fixture" (criteria 18, 23, 38), including the S chip.

### 4.4 Limits: the typed value gives way (D24)

Some sizes depend on another. The rule: the typed or dragged value is the one that gives way; the other one never moves.

- **Holder and clamp**: width and height 1 to 1,000 mm (criterion 2 and 18). A typed value outside is set to the nearest limit with the Action notice of section 4.1.
- **Stop**: each leg 1 to 1,000 mm and at least the thickness. A typed leg below the thickness is set to the thickness: "Leg 1 is 5 to 1,000 mm. Set to 5 mm." Thickness 0.5 to 50 mm and at most the shorter leg, so the L stays an L. A typed thickness above that is set to the limit: "Thickness is 0.5 to 20 mm. Set to 20 mm." when the shorter leg is 20 mm; with both legs at 60 mm the limit is 50 mm: "Thickness is 0.5 to 50 mm. Set to 50 mm."
- **Screw**: each diameter 0.5 to 1,000 mm. The thread is never larger than the head. A typed thread above the head is set to the head: "Thread diameter is 0.5 to 10 mm. Set to 10 mm." A typed head below the thread is set to the thread: "Head diameter is 6 to 1,000 mm. Set to 6 mm." (D26 names the conflict with criterion 37.)
- A drag stops at the same limit and shows the readout of section 0 at the limit ("⌀ 10.0 mm max" for the thread, "⌀ 6.0 mm min" for the head), no notice.

## 5. The look on the canvas (criteria 7, 13, 16, 17, 18, 21 to 23, 37)

### 5.1 Grid (unchanged)

- **Minor lines**: every Spacing mm from the origin, across the whole work area, the edge lines included; `--grid-minor`, 1 device px. **Major lines**: every n-th line counted from the origin; `--lane-line`, 2 device px. Criterion 16 gives the test: 10 mm and major 5 on 600 x 400 mm gives 61 and 41 lines, 13 and 9 of them major.
- **Small spacing on screen**: minor lines fade in between 4 and 12 device px between lines (the page grid's rule of `0090`), and are not drawn below 4 px. Major lines are not faded and stay until 4 px.
- **Over the view grid** of `0090`: the bed grid is drawn above it, so its lines are the stronger ones where the two spacings agree. `0090` is not changed (Proposal D2).
- **Tapes** (criterion 17): on each chosen edge, a band 5 mm wide inside the work area, filled `--ruler-bg` (opaque), 1 px `--ruler-edge` on its inner side. Short ticks (2 mm long, `--ruler-tick-minor`) every 5 mm; long ticks (the full 5 mm band, `--ruler-tick`) every 10 mm; numbers 11px `--ruler-label` every 50 mm on the inner side. Numbers count mm from the grid origin along the edge.
- **Corner**: where two tapes meet, the band of the second edge starts 5 mm inside the first one, so ticks never overlap.
- **Small zoom**: short ticks are not drawn below 4 device px between them. Numbers every 50 mm need at least 40 device px between them; below that the numbers move to 100, 200, 500, 1,000 mm (Proposal D16).
- The tapes are part of the grid: not hit, not selectable, and they belong to the bed, so they are drawn at the drawing's scale (the band thins when zoomed out).

### 5.2 Holder

- **Board**: opaque `--toolbar-bg` ground, 1.5px solid ink outline with casing, drawn at its angle. Name "Holder 1" in 11px ink, inside the top-left corner, drawn only when the board is at least 80 device px wide.
- **Slot**: opaque `--canvas-bg` interior (a hole shows the bed), 1px solid ink outline, rectangle or circle. Name in 11px centred, drawn when the slot is at least 40 device px in both directions.
- **Slot outside its board** (criterion 20): the outline is dashed (4 on, 3 off), so the shape says "not on the board". Nothing else changes. A holder that is made smaller by its handles can leave a slot outside; the slot keeps its size and its centre offset from the board's corner, and this look shows it.
- **Damaged slot** (criterion 33): not drawn, not a snap target.

### 5.3 Clamp

- Solid 1.5px ink outline with casing. Inside: a 45-degree hatch, 1px `--toolbar-icon`, 6 device px between lines, screen space, clipped to the rectangle, no fill: the artwork shows through. Name as the holder's, top-left, when at least 80 device px wide.
- The hatch is ink, not `--engrave-hatch`: the pale orange means Engrave in `0101`, and a clamp is not an engrave (Proposal D3).

### 5.4 Stop

- An L of two legs drawn as a solid bar: fill `--toolbar-icon` at full strength, 1px `--selection-casing` outside. The thickness is the stored one, drawn at least 2 device px. No name on the bar; the name is in the row and the tag.
- Corner at the origin of its box: leg 1 runs along the box's width at the bottom, leg 2 up the box's height at the left (the box is the rectangle's box of the L, section 6.2).

### 5.5 Screw

- Two concentric circles on one centre, **ink only, no fill**: the head, 1.5px solid ink outline with casing; the thread, 1px solid ink outline with casing. Nothing else: no ground, no hatch, no tint, no name on the canvas (the row and the tag carry the name, as the stop). The artwork shows through both circles.
- The thread circle is drawn when it is at least 4 device px across; the head always.
- A screw is not a hole and not a board: its two rings and their own weight are the cue (Proposal).
- Damaged diameters (criterion 33): the field reads its default (thread 6 mm, head 10 mm), the notice of section 13 counts them.

### 5.6 Draw order, bottom to top, inside the bottom layer

Page background; grid minor lines; grid major lines; tapes; holder boards; slots; clamps; stops; screws. Above them, as in `0100`: the placed pages and own items of the page, the origin marker and the placement boxes. The selection box, the handles and the tag are drawn last (DOM for the tag).

## 6. Selection, box, handles, tag and the Select bar

### 6.1 Hit (criteria 8 to 11)

- A press on the outline of an Active fixture selects it alone. Tolerance 4 device px for a mouse; 12 device px on a coarse pointer (Proposal D12; criterion 8 says 4 for all). A screw is hit on either of its two circles.
- A press on a slot's outline selects the holder that holds the slot.
- A press inside a holder, clamp, stop or screw area does not select it: it acts on what lies under. A part on a board stays reachable, and an empty press inside a board starts a marquee on the page.
- A press on another object replaces the fixture selection. Shift+press on another object adds nothing from the fixture.
- A grid is hit only on its outer edge (the work area edge), never inside.
- A fixture of a laser page that is not the active page is not hit (criterion 11): the press activates that page.
- Ctrl+A and marquee select no fixture (criterion 10).
- Active off or eye off on the selected fixture ends the selection; the row stays.

### 6.2 Box and handles, per kind

**Holder and clamp** (the rectangle's set, D22):
- **Box**: the rectangle's bounding-box outline (dashed, 1px `--accent`), in the holder's own frame.
- **Resize**: 8 squircles, corners and edge midpoints, as section 4.3. Edge squircles from a box side of 24px. Hidden, not disabled, when a side is too short.
- **Rotate**: 4 rotate glyphs at the corners, 32px outside, 22.5-degree snap as `0008`. Shift adds the 4 side glyphs. A double-click on a glyph opens the Angle chip; R opens it too.
- **Centre move**: the rectangle's centre handle from a box side of 48px. Double-click opens the Move chip; M opens it too.
- **No skew handles, no corner knobs** (D22). The holder has no corner radius and a skew would make it a path.

**Stop** (D23):
- **Box**: the bounding box of the L (its width is leg 1, its height is leg 2), dashed as the rectangle.
- **Leg 1 and Leg 2 squircles**: as section 4.3, at the free ends on the centrelines. Shown when the leg is at least 24px on screen; the rectangle's edge rule.
- **Thickness knob**: the Parameter handle look (round, centre dot), at the midpoint of the top edge of leg 1. Shown when the shorter leg is at least 72px on screen (the rectangle's knob rule, T).
- **Rotate**: 4 corner glyphs, as the holder. **Centre move**: as the holder.
- **No corner or edge resize** (the stop's size is its legs and its thickness, criterion 23).

**Screw** (criteria 37, 38; D25):
- **Box**: the square around the head circle, dashed.
- **Head knob**: Parameter handle look, on the head circle at 0 degrees (east). Shown when the head is at least 72px on screen (T).
- **Thread knob**: Parameter handle look, on the thread circle at 180 degrees (west). Shown under the same rule.
- **Centre move**: as the holder, from a head of 48px on screen.
- **No rotate handles** (a circle looks the same turned, criterion 14), **no resize, no skew.**
- The two knobs sit at different angles, so they never meet; their hit radii are 12px (Parameter handle), so they stay apart at every size the knobs are drawn.

**Grid**: the box of the work area and nothing else. No handles (criterion 13).

**Slot** (selected by its row or by the holder's outline): its outline as a 2px `--accent` line with `--selection-casing`, and nothing else. No box, no handles.

Handle hit radii are the rectangle's: squircles 16px, rotate glyphs 16px, knobs 12px, centre move 12px. Where two handles compete the nearest centre wins, as the rectangle's rule (`docs/design-system.md`, "Transform handle hit priority").

### 6.3 Tag

- A DOM chip, `aria-hidden`, 20px high, `--toolbar-bg` ground, 1px `--toolbar-icon` at 25 % border, 8px radius, 6px side padding. Kind symbol 12px, 4px gap, the word in 12px `--toolbar-icon`: "Grid", "Holder", "Clamp", "Stop", "Screw" (criterion 8).
- Placed as the clone tags of `0049` (candidates above left, above right, below left, below right; never over a handle or a rotate glyph). No candidate fits: no tag.

### 6.4 Select bar

- **Holder, clamp, stop, screw: no Select bar.** No control of the bar applies to a fixture (no field, no Flip, no Align, no Object to path, no Linked clone, no Radius), so the bar is not shown, rather than shown empty ("hidden, not disabled"). The sizes and positions are the chips of section 4.3 and the Move chip. The fields "X", "Y" and "Angle" of the previous revision are removed (criterion 14; `0008` and `0043`).
- **Grid**: no Select bar (its origin is typed in its block).
- For comparison, the rectangle's bar holds Radius, Link corners and Remove rounding, and no X, Y, W, H or Angle field (section 0).

### 6.5 Active off and eye off (criteria 5, 6)

- **Active off**: not drawn, not hit, not a snap target. Nothing is shown on the canvas for it. The row says so (section 3.3).
- **Eye off**: not drawn, not hit, still a snap target where it snaps (criterion 6; Proposal D11). A screw is no snap target (criterion 40), so its eye has no snap to keep. A snap then shows the snap marker of `0060` at a point the maker cannot see; the marker is the cue.
- Neither switch has a canvas notice: the row carries the state.

## 7. Moving, turning, fitting and snapping (criteria 14, 24 to 27, 40)

### 7.1 Move and turn

- **Holder, clamp, stop, screw**: drag from the outline (tolerance of 6.1), or from inside the selected box where no handle and no other object is hit (Proposal D13). The drag ghost is the `Drag ghost` outline at the committed geometry (`--drag-ghost`). Release commits "Move fixture" as one undo step. A holder moves its slots with it; the slots keep their place on the board.
- Arrow keys nudge: 1 mm, Shift 10 mm (`0044`). The undo steps of a nudge follow `0044`.
- **Turn** (holder, clamp, stop): a rotate glyph with the 22.5-degree snap of `0008`, or the Angle chip (double-click a glyph, or R). Undo "Rotate fixture". A screw does not turn.
- **Resize** (holder, clamp, stop legs and thickness, screw diameters): section 4.3 and 4.4. Undo "Resize fixture".
- **Grid**: no drag. Its origin is set in its block.
- Ctrl+D and copy of a fixture are refused silently in this slice (Proposal D21).

### 7.2 Fit to a slot (criteria 25, 26)

While a placed page (from the page list or the canvas), a clone, a group or an own item is dragged, and an Active holder has a slot it fits (size within 0.1 mm, centre to centre, in its own orientation or turned by 90 degrees; centre within the capture distance of the slot's centre):

- **Slot highlight**: the fitting slot is filled `--accent-hover` with a 2px `--accent` outline and `--selection-casing`. One slot at a time, the nearest. This is the slot cue of criterion 25.
- **Ghost**: when the fit needs the turn, the drag ghost is drawn turned by 90 degrees about its centre (the `Drag ghost` look). Nothing else changes.
- **Drop badge** at the pointer (`Drop badge` look: pill 20px, `--toolbar-bg`, 1px `--toolbar-icon` at 25 %, 12px glyph and word): glyph `Magnet` 12px, word "Fits Slot 2" or "Fits Slot 2, turned". It is the text cue, so the highlight is never the only one. Status region (sr-only, polite) says the same words once for each change of slot.
- **Release**: "Move to slot", one undo step; the item's centre on the slot centre, turned when needed. Fit never changes the item's size. No success notice.
- **No fit** (or no Active holder): no highlight, no badge, a normal drag; the item lands where released (criterion 26).
- **Eye off**: the slot is not drawn, so no highlight; the Drop badge still names it.
- Items that differ by more than 0.1 mm in both orientations do not fit: no badge.
- A screw is not a fit target (criterion 40).

### 7.3 Snap cue (criteria 24, 27, 40)

- Snap points of Active fixtures: grid intersections, slot centre, corners and edge midpoints, board corners and edge midpoints, stop outer edges and corner. A screw has none, not even its centre (criterion 40). Clamps have none (criterion 27 d). Capture, release, hysteresis and Alt rules as `0060`.
- When a moved point snaps, the marker of `0060` shows at the snap point. No fixture-specific cue.
- Clamps are not snap targets: a keep-out is avoided, not snapped to. No cue.
- Switch "Fixtures" in the Snapping Properties panel (section 9.2) turns all of these off with one undo step.

## 8. Layers tree (criterion 12; amends `0106` criterion 17)

- **Fixtures row** at the bottom: glyph `Grid3x3` 16px, name "Fixtures", the group eye (Proposal D14: hides every fixture in view, one undo step per fixture as the eye of `0093`, no output; the children keep their own eyes). No lock, no selection ground, no rename, no drag, no Delete. A press on the row selects nothing and only takes focus, as `0106` says.
- **Children**: one row per fixture in creation order, always listed (no chevron, criterion 12). Rows are 28px, indented one level, glyph 16px (the kind symbol of section 4.0, screw included), name, the word "not on the bed" 11px `--panel-muted-fg` when Active is off, and the eye (24px). Press or Enter on a child selects the fixture on the canvas. Child rows have no rename, no drag, no Delete and no lock.
- **Empty**: the Fixtures row with no children and no text (the empty-page line of `0106` is for placed pages and own items; the Fixtures row always shows).
- Semantics: rows of the existing `treegrid`, `aria-level` 2 for fixtures.

## 9. Machine block and Snapping (criteria 24, 28 to 30)

### 9.1 Profile rows (Frage 175 A; hidden where they do not apply)

In the Machine block, after the profile picker and the work area row:

- **"Fixtures in profile: N"**: 12px `--panel-muted-fg`, a 28px text row, no control. Generic laser: "Fixtures in profile: 0".
- **Save as profile**: a text button, 28px, full width, 1px `--toolbar-icon` at 60 % outline, 14px label. Shown on a user profile, not on Generic laser. No notice.
- **Update profile**: the same look. Shown while the page's set differs from the profile's ("Differs from profile", `0026` criterion 7). Armed as `0026` criterion 7: the armed look of section 3.5 with the name "Press again to update profile". Writes the library record.
- **Reset from profile**: the same look, shown while the set differs. One press, no arming: it is reversible, so it never asks. Undo "Reset fixtures from profile".
- "Save as profile" and "Update profile" are two presses for one stored set, as the spec names them; a veto point (Proposal D18).
- Profile library writes are not in undo (`0026` out of scope).

### 9.2 Snapping Properties (`0060`)

- Row "Fixtures" in the snap targets, after Page: label 13px, `PanelSwitch` (`role="switch"`, name "Fixtures", `aria-checked`, no on or off text). Default on (criterion 24).
- Shown only while the active page is a laser page with the extension on (hidden otherwise, Proposal D15).
- Its state is a document setting, so it is saved with the file and undone as one step.
- Undo "Snap to fixtures on" or "Snap to fixtures off". No notice.

## 10. Keyboard routes

| Action | Route | Notes |
|---|---|---|
| Enter the block | Tab from Place page: the add group (one stop), then the list (one stop, roving rows) | the tab order is the visual order |
| Add a fixture | Left and Right in the add group, Enter or Space | focus stays on the pressed button |
| Select a fixture | Enter or Space on its row | as a press; the row expands if it has block fields |
| Move between rows | Up and Down, Home and End, Page Up and Page Down (5 rows); no wrap | roving focus |
| Inside a row | Left and Right: check box, eye, trash (line 1 order); Space or Enter presses the focused one | the glyph and the name are not stops |
| Rename | F2 on the selected row | the double-click is the mouse route |
| Remove | Delete on the focused row arms the trash; Delete or Enter on the armed row removes | Escape disarms |
| Edit a block field | Tab through the expanded fields, top to bottom; Enter commits, Escape reverts; Up and Down step, Shift 10 | section 4.1 (grid and slots) |
| Pick a slot | Enter or Space on its row | expands its fields |
| Remove a slot | the slot row's trash, same route as above | |
| Canvas: select | press on the outline, or the row, or the Layers child row; no Tab cycling of canvas objects is added in this slice | |
| Canvas: move | M, the Move chip (X, Y, Relative or Absolute, Reference) on a holder, clamp, stop or screw | the rectangle's key, D27 |
| Canvas: size | S, the Size chip: "W" and "H" (holder, clamp); "Leg 1", "Leg 2", "Thickness" (stop); "Head" and "Thread" (screw) | D27 |
| Canvas: rotate | R, the Angle chip on a holder, clamp or stop; none for a screw | D27 |
| Canvas: skew | none; fixtures are not skewed | D22 |
| Canvas: nudge | Arrow keys, 1 mm; Shift 10 mm (`0044`) | |
| Chips | Enter commits, Escape cancels, Tab moves between fields | as the rectangle's chips |
| Canvas: Delete | on a grid: notice "Remove the grid with its trash in the Fixtures block." (refusal look, 8 s); on a holder, clamp, stop or screw: arms its row trash and focus moves to that row (section 3.5); a second Delete removes | Proposal D17 |
| Canvas: deselect | Escape clears the fixture selection and collapses the row; the second Escape is the canvas's own | |
| Ctrl+A, marquee | no fixture selected (criterion 10) | |
| Ctrl+D, copy | refused silently, no step | Proposal D21 |
| Undo | Ctrl+Z | every step of section 12 is one undo |

Notes: no letter key is bound to a kind (the letters are tool letters, as in `0101`). M, R and S are the rectangle's keys for the same chips, not new keys. Every drag has a keyboard route (the chips, the keys, arrows and Layers), so no drag is required (WCAG 2.2 dragging alternative).

## 11. Accessible names (complete)

| Element | Name | State or description |
|---|---|---|
| Add group | "Add fixture" (toolbar) | |
| Add buttons | "Add Grid", "Add Holder", "Add Clamp", "Add Stop", "Add Screw" | Grid absent while the page has one |
| List | "Fixtures" | |
| Row | "<name>, <kind>, on the bed" or "<name>, <kind>, not on the bed" | `aria-posinset`, `aria-setsize`, `aria-selected` on the selected row |
| Active | "Active <name>" (checkbox) | `aria-checked` |
| Show | "Show <name>" | `aria-pressed` = shown (as `0101`) |
| Trash | "Delete <name>" | armed: "Press again to delete <name>", tooltip the same |
| Unknown row | "<name>, unknown fixture" | trash only |
| Rename field | "Name of <name>" | open only while renaming |
| Expanded group | "<name> settings" (group) | grid and slots only |
| Spacing, Major every, Origin X, Origin Y | the word, with its unit: "Spacing (mm)", "Major every (lines)" | the value and its range in the description |
| Tapes on | group "Tapes on"; buttons "Tape on bottom", "Tape on left", "Tape on top", "Tape on right" | `aria-pressed` |
| Size chip, holder and clamp | "Width (mm)", "Height (mm)" | the range in the description |
| Size chip, stop | "Leg 1 (mm)", "Leg 2 (mm)", "Thickness (mm)" | the range in the description |
| Size chip, screw | "Head diameter (mm)", "Thread diameter (mm)" | the range in the description |
| Single-field chips | the same names, one field | opened by the double-click of section 4.3 |
| Angle chip | "Angle (°)" | a holder, clamp or stop |
| Move chip | "X (mm)", "Y (mm)"; the mode "Relative" or "Absolute" | |
| Shape | radiogroup "Shape"; "Rectangle", "Circle" | `aria-checked` |
| Slots heading | "Slots", description "2 of 50" | |
| Slot row | "<name>, <rectangle or circle>, <size>" | description "outside the board" where it applies |
| Slot fields | "Centre X (mm)", "Centre Y (mm)", "Width (mm)", "Height (mm)", "Diameter (mm)" | the range in the description |
| Add slot | "Add slot" | |
| Delete slot | "Delete <slot name>" | armed as above |
| Layers Fixtures row | "Fixtures" (treegrid row) | group eye "Show fixtures" with `aria-pressed` |
| Layers child row | "<name>, <kind>, on the bed" or "not on the bed" | `aria-selected` when selected |
| Tag, Drop badge, Box, tapes, hatch, squircles, knobs | `aria-hidden` | the words are in the row, the chips and the status region |
| Snap switch | "Fixtures" (switch) | `aria-checked` |
| Profile rows | "Save as profile", "Update profile", "Reset from profile" | "Press again to update profile" when armed |
| Selection announcer (`0044`) | "Holder 1 selected, 100.0 by 60.0 mm." · "Stop 1 selected, legs 60.0 and 60.0 mm, thickness 5.0 mm." · "Screw 1 selected, thread 6.0 mm, head 10.0 mm." | the other kinds the same form |
| Status region of the block | polite, sr-only | arming, removal, the fit words |

## 12. Undo steps and notices

One step per gesture: "Add <kind>" (kind in lower case: "Add screw"), "Rename fixture", "Remove fixture", "Set fixture active", "Set fixture inactive", "Move fixture", "Rotate fixture", "Resize fixture" (holder, clamp, stop legs and thickness, screw diameters: by handle, by double-click chip, or by the S chip), "Move to slot", "Set grid spacing", "Set grid major", "Set tape edge", "Set grid origin", "Add slot", "Remove slot", "Rename slot", "Set slot shape", "Set slot position", "Set slot size", "Snap to fixtures on", "Snap to fixtures off", "Reset fixtures from profile". "Show or hide fixture" for the eye (criterion 6; the spec wins over the earlier "no step", lead 2026-11-10). No step for the selection, the expanded row or a focus move. Profile library writes are not in undo.

Removed from the list of the previous revision: "Set holder size", "Set clamp size", "Set stop legs", "Set stop thickness" (now "Resize fixture", criteria 18, 22, 23).

| Text | Where | Role and time |
|---|---|---|
| "Remove the grid with its trash in the Fixtures block." | canvas notice slot (Delete on a grid) | `role="alert"`, 8 s |
| "<Field> is <min> to <max> <unit>. Set to <value>." | Action notice under the field or chip | status, 3 s |
| "Press again to delete <name>. Its <n> slots go with it." | status region (holder only), on arming | polite, sr-only |
| "1 value damaged on Holder 1." / "<n> values damaged on <name>." | canvas notice slot, on opening the file | status, 5 s (over 70 characters) |
| "⌀ 10.0 mm max", "⌀ 6.0 mm min" | the screw's knob readout while a drag is at a limit (section 4.4) | the pointer readout of `Live transform readout`; no notice |

Not used: "A holder holds up to 50 slots." (the add is hidden at 50, D6); "Slot <name> lies outside <holder>." (a permitted state; the row shows it, so no notice); "This page has a grid." (the grid is hidden from the group, D6); any hint line for a handle (customer rule 2026-10-16, D23). No success notice for add, move, resize, fit, snap or profile writes (customer, 2026-10-31).

## 13. Empty, error and large states

- **Empty page**: the Fixtures heading and the five add buttons, nothing else (criterion 1). The Layers Fixtures row with no children.
- **Holder without slots**: "Slots 0 of 50" and an empty list. No text.
- **Unknown kind**: section 3.3. Draws nothing, is not hit, not a snap target, not output.
- **Damaged values** (criterion 33): the field or the diameter reads its default; a damaged slot is skipped; one notice counts the values; the file is not rewritten on read.
- **Old files** (criterion 34): a grid saved by `0093` opens Active, with its tapes on the default edges; nothing is written on open. A stop or holder of the previous revision (typed fields in the file) opens with its stored sizes; the typed fields are simply not shown.
- **Large**: 50 fixtures and 50 slots are windowed (fixed row heights, at most 60 row elements in the DOM for the block); the first rows paint at once; the slot list of an expanded holder scrolls in the tab's one scroll area. Canvas: the grid is one batch of lines per frame, the fade rule removes the minor lines when they are too dense, labels are drawn only at 40 or 80 device px, the fit highlight is one slot at a time, the tag is one for the selected fixture only, the knobs and squircles exist only for the selected fixture. The CPU budgets are criterion 35 (the architect confirms them in `adrs.md`); the UX side adds nothing to them.

## 14. Decisions for veto (Proposal)

- **D1 Colours.** No fixture colour; ink and existing tokens only; the concept's brown tokens are dropped.
- **D2 Grid.** Minor `--grid-minor`, major `--lane-line` at 2 px; the `0090` view grid stays under it, unchanged.
- **D3 Clamp hatch in ink**, not pale orange (orange is Engrave).
- **D4 Symbols** as section 4.0 (Grid `#`, Holder board with two slots, Clamp brackets, Stop solid L, Screw two rings).
- **D5 Row height**: 40px (48 coarse) in the block, amending `0098`'s 36px; 28px in Layers.
- **D6 Hidden at the limits**: the second grid and the 51st slot are removed from the tree (house rule), the reason visible in the grid row and "Slots 50 of 50". Amends criterion 2 ("the button shows why") and criterion 19 (the refusal notice).
- **D7 Active check box** with the line 2 words "not on the bed" and a muted name.
- **D8 Block fields; Line width hidden** as criterion 16 says. `0093` showed it; this removes it.
- **D9 Armed trash** for fixtures and slots, superseding `0093`'s one press. The design system says a reversible action never asks; this follows the customer's armed trash, as decided for operations in `0101` decision 8. Decide both together.
- **D10 Fit cue**: slot fill plus 2px accent outline, a Drop badge naming the slot ("Fits Slot 2, turned"), the turned ghost. No notice on release.
- **D11 Eye off still snaps** (criterion 6). The snap marker shows where. The alternative is that eye off stops snapping too.
- **D12 Hit tolerance**: 4 device px with a mouse, 12 with a coarse pointer.
- **D13 Move from inside**: a selected holder, clamp, stop or screw moves from its box interior where no handle and no other object is hit.
- **D14 Layers group eye** hides all fixtures in view; child rows keep their own eyes; children are rows (amends `0106`'s "no chevron" for this row only).
- **D15 Snap switch** only on laser pages, with an undo step.
- **D16 Tape numbers** move to 1-2-5 steps at small zoom; ticks are dropped below 4 px.
- **D17 Delete on the canvas** arms the row trash (the second Delete removes). On a grid it shows the notice.
- **D18 Two profile presses**: "Save as profile" and "Update profile" both stay, as criterion 28 names them. Merging them is possible. "Reset from profile" is undoable and never asks.
- **D19 No Fixtures section** in the Operations tab in this slice; fixture jobs come with `0103`.
- **D20 Tapes** as four edge toggles, all four on by default (Frage 174 C); numbers count from the 0090 grid's origin corner (Frage 174 B).
- **D21 Ctrl+D and copy** of a fixture are refused silently in this slice.
- **D22 The rectangle as the model, for holder and clamp.** Eight resize squircles with the rectangle's chips: a corner opens "W" and "H", an edge midpoint its one axis ("H" for n and s, "W" for e and w), the fixed point kept, four rotate glyphs, the centre Move chip. **Not taken:** skew (a skew would make the fixture a path) and the corner knobs (a holder or clamp has no corner radius). Veto if the customer wants skew or radius for them.
- **D23 Stop handles and no hint.** Leg 1 and Leg 2 squircles at the free ends (length, with the "Leg 1" and "Leg 2" chips), a Thickness knob (chip "Thickness"), four rotate glyphs, the centre Move chip; no corner or edge resize. The thickness knob sits on the top edge of leg 1, the edge that moves with the thickness; criterion 23 says "outer edge", which must be read so or the knob moves to the inner corner. **No hint chip for any fixture handle** (the rectangle's 600 ms hint is not taken, customer rule 2026-10-16).
- **D24 The typed value gives way.** Thread never above the head, head never below the thread, leg never below the thickness, thickness never above the shorter leg (or 50 mm). The typed value is set to its limit with the Action notice; the other value does not move. Drags stop at the limit without a notice.
- **D25 Screw handles.** Head knob at 0 degrees on the head circle, thread knob at 180 degrees on the thread circle (criterion 38 asks for different angles). Each drag changes its diameter by the pointer's distance from the centre along the knob's axis. Knobs are drawn from a head of 72px on screen (the rectangle's T); the centre move from 48px. No rotate, no resize, no skew. The S chip carries both diameters.
- **D26 Thread never larger than the head. CONFLICT with criterion 37.** Criterion 37 says "a head smaller than the thread is allowed and drawn as typed". This note keeps the rule that the thread is never larger than the head, as the review brief asked, and sets the typed value to the limit, as D24. The product owner or the customer decides; if criterion 37 stands, D24 and section 4.4 change for the screw.
- **D27 M, R and S open the rectangle's chips** for fixtures (the same keys as for any object in the Select tool). This replaces the previous "no keyboard route other than the typed fields". The screw has no R. Not new keys, no letter bound to a kind. Criterion 14 says "M opens the offset entry": that reads as a slip, since M is the Move chip and the Offset tool is O.
- **D28 Five add buttons 44px wide** (236px in total) to fit the row width; the previous 56px buttons would not leave room for Screw. Coarse pointer: 44 x 48px.
- **D29 Screw readouts and words.** "⌀ 6.0 mm" live, "⌀ 10.0 mm max" and "⌀ 6.0 mm min" at a limit (the rectangle's "r … max" pattern). No size text in the Fixtures row; the announcer carries the sizes.
- **D30 Screw defaults and ranges**: thread 6 mm, head 10 mm (an M6 screw, criterion 37, Proposal); 0.5 to 1,000 mm each; round only, no hex, no countersink (customer, 2026-11-10).
- **D31 No Select bar for fixtures.** The bar is not shown with a fixture selected; the rectangle has no X, Y, W, H or Angle field either. Removes the X, Y and Angle fields of the previous revision.

Changes this asks of the product owner (the spec is theirs): criterion 2 (five buttons at 44px, D28); criterion 14 ("M opens the offset entry" reads "the Move chip"; "rotate handle" reads "the four corner rotate glyphs"; the stop and screw handle sets, D23, D25); criterion 23 (the thickness knob on the top edge of leg 1, D23; the limits of D24); criterion 37 (D26, a conflict to decide); criterion 38 (the knob positions are these, D25); criterion 19 (the refusal notice, D6, unchanged from the last revision).

## 15. Review checklist for the implementer

- Add group: five buttons 44 x 40 with symbol over word, toolbar keys, Grid hidden while a grid exists, no popup.
- Rows: 40px; check box, glyph, name, eye, armed trash; line 2 with kind, count and "not on the bed"; no colour, no order arrows.
- Block fields: grid and slots only; bar number fields, pairs of 118px, Enter, blur and Escape, Action notice under the field, undo per commit. No size or position field for holder, clamp, stop or screw.
- Slots: rows with shape symbol and "Outside the board" text, shape switch keeps the size, Add slot hidden at 50.
- Canvas: ink only; grid minor and major, fade rule, tapes with ruler tokens and corner rule; boards tinted with `--toolbar-bg`; slots as holes; clamp hatch in ink; stop as solid bar; screw as two rings with no fill; casing under every line; draw order of section 5.6.
- Handles: holder and clamp as the rectangle (8 squircles, 4 rotate glyphs, centre move, no skew, no knobs); stop with two leg squircles and the thickness knob; screw with two knobs at 0 and 180 degrees; knobs from 72px, centre move from 48px, no hint chip.
- Chips: the rectangle's Transform entry chip and Move entry chip; "W" and "H", "Leg 1", "Leg 2", "Thickness", "Head", "Thread", "Angle"; the limits of section 4.4 with the Action notice; Enter, Escape, Tab.
- Keys: M, R, S open the chips on a selected fixture; no R on a screw.
- Select bar: not shown for a fixture; the grid has none.
- Selection: outline hit (4 or 12 px), slot selects holder, inside not hit, a screw hit on either circle; box and handles only; grid box only; tag placed as clone tags.
- Active off and eye off: not drawn, not hit, selection ends; eye off still snaps where a snap exists; a screw never snaps.
- Fit: slot highlight, Drop badge words, ghost turned, one step "Move to slot", no notice; eye off keeps the badge.
- Layers: Fixtures row with its group eye and child rows (screw included); press selects; no rename, drag or Delete there.
- Profile rows and the snap switch as section 9.
- Keyboard and names as sections 10 and 11; no tips, no hint chips, no success notices.
- Undo labels as section 12: "Resize fixture", "Move fixture", "Rotate fixture", "Add <kind>".
- Large: 50 fixtures and 50 slots windowed; grid draw as one batch.

## Links

Spec: `specs/0102-laser-fixtures/specification.md` (criteria 1 to 40, Fragen 171 to 175; revised 2026-11-10 after the review of PR #313). Model: `specs/0101-laser-operations/ux-notes.md`. Rectangle: `specs/0008-object-transform-refinements/`, `specs/0019-multi-object-transform/`, `specs/0072-pen-angle-snap-and-typed-values/`, `specs/0073-parameter-handle-clone/`, `docs/design-system.md` (section 0 rows). Armed trash: `specs/0081-inline-delete-confirm/ux-notes.md`. Fixtures block and eye (superseded here): `specs/0093-laser-workbench-shell/ux-notes.md` section 4. Layers Fixtures row (amended): `specs/0106-move-versus-edit/ux-notes.md`. Snap marker: `specs/0060-snapping/ux-notes.md`. Clone tag placement: `specs/0049-linked-clone/`. Concept: `specs/0063-manufacturing-workbenches/concept-output-pages.md` §6, sketch g of `concept-sketches.html`. Design system rows to add at acceptance: "Fixture symbols (`0102`)", "Fixture block (`0102`)", "Fixture canvas look (`0102`)", "Fixture handles (`0102`)"; no new token is proposed. Customer rules: no tips, no hint chips, no success notices, no popups in the panel, no invented colours, the rectangle as the model, no typed size or position field for holder, clamp, stop or screw (2026-11-10).
