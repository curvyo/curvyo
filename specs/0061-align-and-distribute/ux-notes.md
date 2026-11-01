# Align and distribute — UX notes

Status: lead draft from the spec's defaults (ALIGN-KEY A, ALIGN-GAP A,
ALIGN-GRID A), 2026-10-30. Checked against `docs/design-system.md` (rows
"Select bar layout", "Flip buttons", "Bar number field", "Action notice").
New interaction details are **(Proposal)**: reversible, vetoable at the demo.

## Placement and visibility (criterion 1)

- Two groups in the Select bar, **after the Guide button, before the kind
  groups**: `role="group"` "Align" (six buttons) and `role="group"`
  "Distribute" (four buttons and the gap field). 28 x 28 px icon buttons,
  4 px apart, 16 px glyphs at 1.5 px absolute stroke, `currentColor`
  (`--toolbar-icon`), hover `--editor-accent-hover`, focus-visible 2 px
  `--editor-accent` ring with 1 px `--toolbar-bg` offset — the Flip buttons'
  look, plain command buttons (no `aria-pressed`). Groups wrap whole.
- **Extended 2026-10-30 (customer):** the groups are in the bar **in every
  selection state of the Select tool**, nothing selected included — with
  nothing selected they act on the **active page's objects** (align onto the
  page, distribute evenly across it), so the bar does not come and go with
  the selection. Shown on an empty page too; a press changes nothing (criterion 15).
- Tooltips (Radix, 400 ms, `side="bottom"`, max 260 px): line 1 semibold the
  name, line 2 `--panel-muted-fg` the rule. No shortcut is shown (criterion
  13: none are assumed).

## Glyphs (criteria 2, 6)

Custom 16 x 16 SVGs in the "Command glyphs" family, at most three shapes:
bars of two different heights (4 and 7 wide, stroke, no fill) and the
reference as a **dashed** line (2.5 on / 2 off, like the Flip glyph's axis).

- Align left / right: the dashed vertical reference at x = 2.75 (right:
  13.25), the two bars' near edges on it.
- Align horizontal centres: the dashed vertical line at x = 8 through both
  bars' centres.
- Top / vertical centres / bottom: the same three drawings turned 90°.
- Distribute gaps: three vertical outlined bars (2.5 wide, 1.5 px stroke, no
  fill) with equal gaps between two dashed vertical guides at the glyph's
  edges (UI review 2026-11-01 item 2: the solid bars were heavier than Align,
  Flip and Grid).
- Distribute centres: three outlined bars of different heights on one
  horizontal dashed guide through their centres — the centres are evenly
  spaced.
- The vertical pair is the horizontal drawing turned 90°, so the four are
  told apart at a glance, gap and centres by the dots.

## Names and rules (criteria 2, 6, 8)

| Button | Name | Rule line |
|---|---|---|
| 1 | Align left edges | Left edges land on the reference's left edge. |
| 2 | Align horizontal centres | Centres land on the reference's horizontal centre. |
| 3 | Align right edges | Right edges land on the reference's right edge. |
| 4 | Align top edges | Top edges land on the reference's top edge. |
| 5 | Align vertical centres | Centres land on the reference's vertical centre. |
| 6 | Align bottom edges | Bottom edges land on the reference's bottom edge. |
| 7 | Distribute horizontal gaps | Equal gaps between the objects, horizontally. The outermost stay. |
| 8 | Distribute horizontal centres | Equal distances between the centres, horizontally. The outermost stay. |
| 9 | Distribute vertical gaps | As 7, top to bottom. |
| 10 | Distribute vertical centres | As 8, top to bottom. |

The reference the buttons act on is named in every align tooltip's rule only
implicitly ("the reference"); the maker sees it: with a key object it is that
object, with one selected object the page, else the selection's box. No text
labels, no dialog (criterion 1).

**Extended 2026-10-30 (customer):** with nothing selected the active page is
both the set to act on and the reference, so the rule line says so instead of
naming a selection — one line each, same structure:

| Button | Rule line with nothing selected |
|---|---|
| 1–6 | "Nothing selected: every object of the page onto its left edge." (and the other five edges and centres) |
| 7, 9 | "Nothing selected: the page's objects evenly across the page, equal gaps." |
| 8, 10 | "Nothing selected: the page's objects evenly across the page, equal centres." (9, 10: "down the page") |

Distributing on the page spreads over the **page's whole extent**: n objects
get n + 1 equal gaps (page edge to first, between objects, last to page edge);
for equal centres the n centres divide the page into n + 1 equal parts. This is
the way Illustrator's distribute-to-artboard works. A typed gap spaces from the page's
start edge and may reach past the end edge; the field is the same field.

## Gap field (criterion 8)

- One number field **right of the four Distribute buttons**, 84 px wide, the
  "Bar number field" look: the visible label "Gap" **outside, left of the
  field** (the Radius field's structure), 28 px high, white ground, 14 px
  tabular, right-aligned, the fixed `mm` suffix inside the right edge with
  32 px of right padding on the value, so value and unit never overlap — the
  unit is always visible. Accessible name "Gap, millimetres".
- Empty (default) means "keep the ends": equal gaps between the outermost
  objects. A typed value is the gap in millimetres; the outermost **left**
  object stays while everything to its right is spaced from it (the same for
  a vertical distribute, top stays). **Amended 2026-10-30:** with a typed gap
  two objects are enough — the second is spaced from the first (criterion 8
  needs no ends); without one, three or more (criterion 7).
- Enter distributes the horizontal gaps with the typed gap; the value stays
  for the next press (session state, never saved). Escape restores the value
  the field had and writes nothing, as in every bar field. Text that is not a
  number is refused with "Enter a number" and a negative value with "Must be
  0 or more." (**Proposal**; the field keeps the typed text and marks itself
  invalid, nothing is distributed.)
- A distribute press ignores the field's value only when it is empty; there
  is no separate apply button.

## Key object (criterion 5, Proposal)

- **Set or clear:** with the Select tool, a **Ctrl-click on an already
  selected object** toggles it as the key object. Today a Ctrl-press on a
  selected object starts a copy-move that a click without movement never
  commits, so the gesture takes nothing away: a Ctrl-drag still copies. The
  mark is set at the press, so it is visible during the drag it might
  accompany; a second Ctrl-click on the same object clears it.
- **The mark:** the key object's own box, drawn as the selection box's dashed
  `--accent` outline at twice the line width, over its normal selection box
  (a group is marked by the box around all of it).
- **Escape** clears the key mark first and keeps the selection; a further
  Escape clears the selection, as the cascade of `0010` orders.
- The key is never stored, never undoable, and forgotten the moment its
  object is no longer selected. With one selected object the key is
  meaningless (the page is the reference, criterion 4) but harmless.

## Feedback (criteria 7, 12)

- A press that moves something writes its one history step and keeps the
  selection; no toast, no announcement (the canvas shows the result, as with
  Flip's success case; the History tab names the step, criterion 12).
- A press that changes nothing (criterion 3, criterion 7 with fewer than
  three objects) shows no message: a no-op is permitted and silent.
- Keyboard: the buttons are Tab stops in bar order and activate with Space or
  Enter (criterion 13); a mouse press returns focus to the canvas, a key
  press stays on the button (the Flip group's rule).

## Grid and page reference (2026-11-01)

Customer additions: **Arrange in a grid** (rows, columns, gap; one history
step) and align and distribute relative to the active page with nothing **or
one object** selected. Same bar, no new toolbar, no dialog, no hint labels.
Behaviour marked **(Proposal)** is vetoable at the demo.

### Grid group (placement, glyph, names)

- A third `role="group"` "Grid" in the Align/Distribute row, **after the
  Distribute group**, behind the same 1 px 20 px 25% divider. Same structure
  as Distribute: the 28 x 28 command button first, then the fields. Order and
  Tab order: button, Rows, Columns, Gap. The group wraps whole, like the other
  two; nothing is added to the bar's other rows.
- Button: the Align buttons' look (`BUTTON_CLASS`), plain command, no
  `aria-pressed`. Glyph in the same 16 x 16 family: four 4.5 px squares in a
  2 x 2 block at 2 / 9.5, 1.5 px absolute stroke, no fill, `currentColor`.
  Accessible name and tooltip name: "Arrange in a grid".
- Fields, "Bar number field" look and the Gap field's structure: visible label
  **outside, left** (Rows, Columns, Gap), 28 px high, white ground, 14 px
  tabular, right-aligned. Rows and Columns 44 px, no suffix (they take 1-2 digits). Gap 72 px with the
  fixed `mm` suffix inside the right edge and 32 px right padding. Accessible
  names "Rows", "Columns", "Grid gap, millimetres" (the Distribute field stays
  "Gap, millimetres"; the groups tell them apart). The group is about 330 px
  (UI review 2026-11-01 item 4).

### Field behaviour

- **Linked rows and columns.** The field the maker types in drives; the other
  shows the derived count as its placeholder in `--field-derived-fg`
  (`#767680`, lighter than typed text so it does not read as entered; UI
  review 2026-11-01 item 1; not selectable text): typed columns c gives rows = ceil(n / c); typed rows
  likewise; neither typed gives columns = ceil(sqrt(n)) and rows = ceil(n /
  columns) (**Proposal**). The derived value follows the selection live. Typing
  into the derived field makes it the driver and the former driver becomes
  derived; clearing a field returns it to derived. The last row may be
  partly empty (reading order, left first).
- **Gap** empty means 0 mm (placeholder "0"), the same gap between rows and
  columns, between cell edges. The Distribute Gap field is unrelated.
- **Enter** in any of the three fields arranges (the button does the same).
  Values stay for the next press (session state, never saved, as Gap).
  **Escape** restores the value the field had on focus, writes nothing and
  blurs, as in every bar field. A press elsewhere does the same as Escape.
- **Invalid input** keeps the typed text, marks the field `aria-invalid` with
  the 2 px `--field-invalid` ring and a 12 px message chip under it (the Gap
  field's chip, `role="status"`), and arranges nothing: Rows, Columns: "Enter a
  whole number" (text, 1.5, empty after typing a sign) and "Must be 1 or more."
  (0, negative); Gap: "Enter a number", "Must be 0 or more." (the words of
  Distribute's Gap).
- Result (**Proposal**): objects in reading order of their current boxes (top
  row first, left first), each centred in its cell, cells as large as the
  largest object's box, the block's top-left at the top-left of the set's
  current box. One history step "Arrange in a grid"; the selection stays; no
  toast. A press that changes nothing is silent (criterion 3).

### Page reference

- **Which mode:** 0 selected: the page's objects are both set and reference
  (2026-10-30). **1 selected: the object is the set, the page is the reference**
  (align already did this; distribute now does too). 2 or more: unchanged
  (selection box or key object). Align and Distribute stay in the bar in every state, an empty page included
  (criterion 15); Grid shows only from two selected objects (criterion 18).
- **Distribute with one object (Proposal):** the page's whole extent is the
  span, so one object sits **centred across it** (equal gaps to both edges;
  for centres its centre on the page's centre). A typed gap puts its start edge
  that gap from the page's start edge, as with more objects. The four rule
  lines below say so; Distribute is therefore shown from one object, and the
  Gap field is shown with it.
- **Tooltip rule lines**, same structure (name semibold, rule muted), by mode:

| Buttons | One object selected |
|---|---|
| Align 1-6 | "Its left edge onto the page's left edge." (right, top, bottom alike; centres: "Its centre on the page's horizontal centre.", "... vertical centre.") |
| Distribute 7, 9 | "Centred across the page, equal gaps to both edges." (9: "down the page") |
| Distribute 8, 10 | "Centred across the page." (10: "down the page") |

  Nothing-selected lines stay as written above. With two or more selected the
  rule lines stay as the table at the top ("the reference"); they are not
  reworded.
- **Canvas cue:** while a button whose reference is the page (nothing or one
  object selected) is hovered or keyboard-focused, the **active page's frame is
  drawn in the key object's mark**: dashed `--accent` outline at twice the
  selection line width, above the page, below the selection box, no fill, no
  label. It disappears on pointer leave, blur or Escape, instantly (no motion);
  a press leaves it while the pointer stays. It is a canvas overlay: it moves
  nothing, takes no hit, and is clipped with the viewport (nothing drawn when
  the frame is off-screen). The Grid button and its fields never draw it: the
  grid does not use the page.

### Disabled states

- **Align, Distribute buttons:** none disabled; they stay shown on an empty
  page (criterion 15). A press with nothing to move is silent.
- Keyboard: buttons and fields are Tab stops in bar order; a mouse press on the
  button returns focus to the canvas, a key press stays (the Flip rule); Enter
  in a field leaves focus in the field.

## UI review (2026-11-01)

Verdict: **APPROVE WITH CHANGES.** Everything the customer asked for works in the real
browser (preview build of `story/0061-align-and-distribute`, 1440 x 900, narrow 700-1100):
align 3 objects left (one history step "Align left", undo and redo with the toast
"Undid: ..."), distribute with nothing, one and two objects, a second page at x = 239.7 mm
(only that page's objects move), grid 2 x 3 with gap 5 (reading order, equal cells, one step
"Arrange in a grid", selection stays), `frontend/tests/browser/align-distribute.cjs` PASS, no
page errors. Tab order (Align 1-6, Distribute 1-4, Gap, Grid, Rows, Columns, Grid gap), Enter
keeping focus in the field, Escape restoring and blurring, the refusal chips ("Must be 1 or
more.", "Enter a number") and the page cue on hover (dashed frame, gone on leave) match these
notes. Screenshots: `target/0061-browser/ux/` (best: `53-page2-grid-2x3-gap5.png`,
`12-tip-align-one-cue.png`, `22-rows-invalid.png`, `20-glyphs-align-dist.png`).

Required changes:

1. **Derived Rows/Columns look like typed values.** The placeholder colour in `BarField`
   (`frontend/src/components/AlignGroup.tsx`, `placeholder:text-[var(--panel-muted-fg)]`) is
   `#5A5A5F` against the typed `#3A3A3F`; in `13-bar-six-selected.png` the derived "2 / 3" read as
   entered numbers. Add a token `--field-derived-fg` (light `#767680`, 4.5:1 on white; dark
   theme counterpart in `docs/design-system.md`) and use it for these placeholders only.
2. **Distribute glyphs are solid blocks.** In `20-glyphs-align-dist.png` the four Distribute
   icons are filled and visibly heavier than Align, Flip and Grid (1.5 px outlines); the notes
   ask for outlined bars. Redraw them as outline bars with the dashed guide
   (`AlignGroup.tsx` glyph components) so the three groups read as one family.
3. **Narrow bar: the Distribute Gap field leaves the bar.** At 700 px width the Gap field spans
   x 320-404 while the bar ends at 376 (`40-narrow-700.png`). Give the bar a minimum content
   width of its widest group (about 260 px) or let that group wrap its field below the buttons
   (`SelectToolbar.tsx` bar container, `GROUP_CLASS`); nothing may overflow the bar background.
4. **Grid adds a third bar row at 1440 px.** With 2+ objects selected the bar is three rows high
   and "Linked clone" sits alone on row 3, covering about 200 px of the canvas top, where the
   objects being arranged often are (`14-six-selected.png`). Do not accept a lone trailing
   button: keep Linked clone with the Convert group on one row (wrap by group, not inside it)
   or set the Grid fields at 44/44/72 px so Grid fits row 2 (Rows/Columns take 1-2 digits).

Notes the lead must settle (not code changes by default):

5. **Spec and notes disagree on when Grid shows.** Criterion 18 and `useAlign.ts` (`grid.shown`
   = 2 or more selected) hide Grid with nothing or one object selected; the "Grid and page
   reference" notes above show it always (disabled with a reason, or arranging the page's
   objects). The code follows the spec. Recommendation: the spec wins; delete the "Disabled
   states" Grid paragraph here so no dead text remains. Grid on the whole page with nothing
   selected would be a new behaviour: a question to the customer, default no.
   **Settled by the lead 2026-11-01:** the spec wins. Grid shows only from two selected objects; the Grid paragraph under "Disabled states" is deleted.
6. **Stale statements in these notes.** Distribute on the page is n + 1 equal gaps (criterion
   16), not "first box on the start edge"; Rows and Columns can never both hold typed text
   (typing one clears the other), so "both typed" and "Not enough cells for n objects." are
   unreachable and `GRID_NOT_ENOUGH_CELLS` is dead unless a path reaches it. Amend the notes,
   and the tester should confirm the string has no caller.
   **Settled by the lead 2026-11-01:** amended above (distribute on the page: n + 1 equal gaps, criterion 16; "both typed" and "Not enough cells" removed; "hidden on an empty page" replaced by criterion 15).
7. **Key object not verified.** After Select all, a Ctrl-click on a selected object's outline
   showed no accent mark and Align used the selection box (`64-key-zoom.png`, `66-key-aligned.png`).
   I could not tell a missed hit from a missing mark; the tester should check it with an
   object that has a fill. Not a 0061 blocker if it was accepted in an earlier slice.

Not a finding: Align and Distribute stay shown on an empty second page (criterion 15 says so);
the notes' "hidden on an empty page" is the stale line, amend it with item 6.
