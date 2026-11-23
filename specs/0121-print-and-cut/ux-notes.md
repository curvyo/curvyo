# 0121 Print and cut: UX notes

ux-engineer, 2026-11-23. Written from `specification.md` (slice 1, criteria 1 to 22; Fragen 328 to 332 at
their default A), the placed page of `0100` (frame, name tag, Style block, refusals), the Operations tab of
`0101` and `0029` (sections, warn lines, output group), the Machine controls footer of `0118` (the greyed
Home with its cause), the Connector and Controller blocks of `0064` and `0127` (the position read), the
group entering of `0050`, the test grid of `0111` (Frage 206 A: an object that only moves) and
`docs/design-system.md` ("Hidden, not disabled", "Armed action", "Canvas notice slot", "Action notice", "A
preview is never a commit", "No notice after a success", "Casing over artwork", "Bar number field", "List
row"). Drafts: `docs/design-drafts/2026-11-print-and-cut/` (01 to 03). Customer rules applied: no invented
colours (ink, muted ink, the selection blue, the guide green for what never runs), no operating tips, labels
that name a control or a state only, plain number fields and no sliders, units inside the fields, edits in
place and no popups in the panel, symbol buttons with tooltip and accessible name, compact. **(Proposal)**
items are vetoable at the demo; section 11 lists them as P1 to P22, section 12 the spec deltas E1 to E12 for
the product owner. "Ink" is `--toolbar-icon`, "muted" `--panel-muted-fg`, "column" the panel's content
width: 244 px (280 less the paddings and a thin scrollbar; the 256 px Style column of `0135` leaves 12 px
spare, every row below is measured at 244).

## 0. What is reused

| Part | Reused from | Change |
|---|---|---|
| Entry on the Drawing page | the Fit to content button of the Document tab (244 x 28 outline text button) | a second button under it |
| Marks object selection | the group box and entering of `0050`, the move-only test grid of `0111` | no resize or rotate handles |
| Marks settings | the Style tab block of a placed page (`0100`), the Params length field | block "Registration marks" |
| Print and cut group | the Fixture jobs section header (fold button with a right-hand summary), the Output group's `h3` look | new section |
| Measured X and Y | the Params length field (28 px, unit inside the right edge, Enter or blur commits, Escape reverts) | column labels above instead of inside |
| Take head position | the Controller block's position read (`0127`), the greyed Home of `0118` (cause as tooltip line 2) | one 24 x 28 symbol button per row |
| Warnings | the `OutputChecks` warn line (`TriangleAlert` 16 px, ink, never `--destructive`) | three texts |
| Fit preview | "A preview is never a commit": the hollow `--preview-new` outline with casing | the fitted sheet frame |
| Marks on the laser page | `--guide` (`#166534`), the colour of what never runs (`0046`) | alias `--reg-mark` at 60 % |

## 1. Principle

The marks are ordinary artwork the maker places and moves; the alignment lives on the laser page and only
moves the placed sheet. The measuring is a small table the maker fills top to bottom, by typing or by one
press per mark while standing at the machine; the result is read as numbers before anything moves, and
Apply is one undoable placement change. Nothing is hidden behind a dialog, nothing runs on its own.

## 2. Drawing page: adding the marks (criterion 1)

- **Place (P1, E1).** The spec's "Insert group" does not exist: the app has no menu (`0051`), the rail is
  full (row "Tool rail architecture": "a new family of commands goes to the Select bar or the Properties
  panel") and the Select bar needs a selection. The entry is a button in the **Document tab of a Drawing
  page**, the tab shown with nothing selected: **Add registration marks**, the Fit to content look (244 x 28,
  1 px ink at 60 % outline, `rounded-[5px]`, 14 px ink label, no glyph), 8 px under Fit to content (in its
  place while Fit to content is absent on an empty page), above the Background block. Absent on Sketch,
  Laser, "Laser (off)" and "Unknown kind" pages. Never greyed: a second marks object is allowed
  (criterion 6). No tooltip (a text button names itself), no shortcut (P2: a once-per-sheet action;
  `Shift+Ctrl+D` and Tab reach it).
- **Press.** Adds one object "Registration marks" with marks 1 to 4 at the corners of the artwork box (all
  objects on the page without marks objects; the page when empty), 10 mm inside it (E3), in one undo step
  "Add registration marks". Numbers run **clockwise from the top-left**: 1 top-left, 2 top-right,
  3 bottom-right, 4 bottom-left (P3). The new object is selected with the Select tool, the Style tab shows
  its block (the existing tab switch on selection), focus goes to the canvas. No notice (the object is the
  feedback).
- **Too small a box.** When the box is under 20 mm on a side the four marks would overlap: they are placed
  at the corners of the box itself (no inset) and the press still succeeds; the maker moves them. No
  refusal (P4).

## 3. The marks object on the Drawing page (criteria 1 to 3, 6; draft 01)

### 3.1 Look

- Each mark: a circle of 6 mm diameter, a crosshair of 8 mm (two lines through the centre, 1 mm past the
  circle on every side), line width 0.25 mm, black `#000000`, no fill; drawn as artwork, true size, in
  every view of a Drawing page, and exported as drawn.
- **Number:** one digit 3 mm high (cap height), black, upright, in the app's UI sans as outlined shapes
  (the export does not depend on a font; the architect names the glyph source). Its box sits up and to the
  right of the centre, its lower-left corner 3.5 mm right of and 3.5 mm above the centre (P5): clear of the
  circle (radius 3 mm) and of both arms at any size; the spec's "4 mm beside" read as the nearest distance
  to the digit, about 4.9 mm to its corner. The same side for all four marks, so the maker reads them
  alike on the print; at 10 mm inside the box the digit stays inside it.
- Marks are always upright (no rotation of a mark or of the object, P6), so the crosshair lines up with the
  sheet edges on the print and the digit is always readable.

### 3.2 Selecting, entering, moving

- **Object level.** One press on any mark selects the whole object: the dashed `--shape-handle-stroke`
  selection box around all marks and digits, **no resize or rotate handles** (P6, the test grid rule of
  Frage 206 A: a scale would move marks the maker placed). Drag, arrow keys, typed X and Y in the Select
  bar and Align move it; Copy, Paste, Duplicate and Linked clone (`0049`) work as for a group (criterion 6).
  The Select bar shows X, Y and the reference points only (W and H absent, rotation absent, flip buttons
  greyed at 40 % with their Tab stop, the recorded exception).
- **Entering.** Double-click on a mark, or Enter with the object selected, enters it (the `0050` entering,
  the read-only crumb "Registration marks", outside dimmed). Inside, a press selects one mark: its selection
  box is the circle's square plus its digit, no handles; drag and arrows move it (Shift 10x); the number
  stays (criterion 1). Hover box per mark. Several marks by Shift+press or a marquee inside. Escape or a
  double-click outside leaves.
- **Layers tab.** One row "Registration marks" with the Lucide `Crosshair` glyph, expandable like a group
  to rows "Mark 1" to "Mark 4" (number order, not stacking order; rows cannot be renamed or reordered).

### 3.3 The Registration marks block (Style tab)

Shown when the marks object or a mark inside it is selected; it replaces Stroke, Fill and Markers, which
are absent for this object (P7: the marks are always black). `role="group"` named "Registration marks".

| # | Part | Size |
|---|---|---|
| 1 | Header "Registration marks" 14 px semibold; right-aligned muted "4 marks" (or "Mark 3 of 4 marks" for one entered mark) | 24 |
| 2 | **Size** field: label inside left, "mm" inside right, 148 px (review: 120 px clipped the value of Line width); the circle's diameter, 3 to 20 mm (default 6); the crosshair is the size plus 2 mm, the digit half the size (E9) | 28 |
| 3 | **Line width** field, 148 px, 0.05 to 1 mm (default 0.25, the spec's range) | 28 |
| 4 | **Add mark 3** (244 x 28 outline text button; the label names the lowest missing number); absent with 4 marks | 28 |

- Fields: the Params field rules (Enter or blur commits one undo step "Registration mark size", Escape
  reverts; out of range: 2 px `--field-invalid`, `aria-invalid`, "Between 3 and 20 mm" under the field,
  nothing written). Up and Down 0.5 mm (Size), 0.05 mm (Line width).
- **Add mark n** puts mark n at its nominal corner (the clockwise order of section 2, the artwork box at
  the time of the press, 10 mm inside), one undo step "Add registration mark", the new mark selected inside
  the entered object, focus stays on the button (it now reads "Add mark 4" or is gone; when gone, focus goes
  to the Line width field).

### 3.4 Deleting a mark (criterion 3; E2)

- With 3 or 4 marks, Delete on a selected mark removes it (one undo step "Delete registration mark"); the
  other numbers stay (1, 2, 4).
- With 2 marks, Delete on a mark is **refused** (P8, E2): refusal alert 8 s in the canvas notice slot
  "Registration marks need at least 2 marks." and nothing changes. The object as a whole is deleted from the
  object level (Delete with the object selected, one step). Reason: the spec's "Delete on the last two
  removes the object" deletes two things the maker did not point at; a refusal with the reason is the
  `0100` pattern ("A placed page stays on its laser page.").
- A selection of several marks that would leave fewer than 2: refused with the same alert.

## 4. Laser page: the Print and cut group (criteria 7 to 14, 16, 18, 19; draft 02)

### 4.1 Where it sits and when

- **Operations tab of the laser page, between the Fixture jobs section and the Output group** (P9). The
  alignment decides where the job lands, so it sits above the output it changes and below the lists the
  maker builds first. It comes and goes as a whole (criterion 7: absent, not disabled, while no placed
  Drawing page on this laser page has a marks object); the Output group moves down by its height, which is
  a change of the page's content, not of a press.
- **Header** (the Fixture jobs header): a 28 px full-width fold button, `ChevronDown` / `ChevronRight`
  16 px, "Print and cut" 14 px, right-aligned 12 px muted tabular summary: "Not measured", "1 of 4
  measured", "Error 0.21 mm", "Error 0.74 mm" with a 14 px `TriangleAlert` before it in the warn state,
  "Too close" in that state. `aria-expanded`. Open when it first appears; open or folded is remembered per
  laser page for the session, not in the file (P10).

### 4.2 Anatomy (column 244 px, 4 px between rows unless stated)

| # | Part | Size | When |
|---|---|---|---|
| 1 | Header (4.1) | 28 | always |
| 2 | **Sheet** line: label 12 px muted "Sheet", the placed page name 13 px ink ("Coaster"), cut at the column with the whole name as tooltip; second line 12 px muted "Registration marks, 4 marks"; with a second marks object a third line "Registration marks 2: not used" (criterion 6) | 16 + 16 (+16) | always |
| 2a | With two or more placed sheets with marks the Sheet line is a full-width outline button with `ChevronDown`, opening the inline listbox "Sheet" (the "Place page" rows: 36 px, name and "4 marks" muted, at most 6 then it scrolls) (P11, E6) | 28 | 2+ sheets |
| 3 | Column labels: "X" and "Y" 12 px muted, each over its field's left padding | 16 | always |
| 4 | One **mark row** per mark of the sheet's marks object, number order, plus a row for each measured mark that is gone (4.4) | 28 each | always |
| 5 | Hairline (1 px ink at 25 %), 8 px above and below | 17 | |
| 6 | **State line** (4.6) or the fit block | 16 to 64 | always |
| 7 | **Tolerance** field, 148 px, label inside left, "mm" inside right (criterion 13) | 28 | 2 or more measured |
| 8 | Warn lines (4.7), each 16 px or two lines of 16 | 0 to 64 | when they apply |
| 9 | **Placement line** (4.8), 12 px | 16 | after an Apply |
| 10 | Button row: **Apply** and **Reset placement**, two outline text buttons 120 x 28 each, 4 px apart | 28 | always |

Open with 4 marks and a fit: 28 + 32 + 16 + 4 x 28 + 17 + 48 + 28 + 28 + 16 rows plus gaps, about 360 px;
folded 28 px.

### 4.3 A mark row (criteria 7 to 9)

Left to right, `role="group"` named "Mark 1":

| Slot | Width | Content |
|---|---|---|
| Number | 14 | the digit, 13 px semibold ink, centred; `aria-hidden` (the group name carries it) |
| X field | 86 | white field, 28 px, value 13 px tabular right-aligned, "mm" 12 px muted inside the right edge, no label inside (the column label above carries it) |
| Y field | 86 | the same, 4 px after X |
| Take head position | 24 | Lucide `LocateFixed` 16 px, 24 x 28, tooltip "Take head position" (above, 400 ms) |
| Clear | 24 | Lucide `X` 16 px, 24 x 28, tooltip "Clear"; absent while both fields are empty (the last slot, so nothing moves, P12) |

14 + 4 + 86 + 4 + 86 + 2 + 24 + 24 = 240 px. The X field holds seven characters ("-123.45", "1234.50")
beside its unit; longer values scroll inside the field (measure at build; if a 4-digit negative value
clips, the unit moves to the column labels, "X mm", P13). **Without Take head position** (no connector,
browser build) the two fields grow to 98 px each. Coarse pointer: rows and buttons 36 px high, widths
unchanged.

- **Fields** (criterion 7): plain text entry, `inputmode="decimal"`, accepts a minus sign and a decimal
  point or comma; shown with two decimals, trailing zeros kept ("41.20") so the columns line up. Enter or
  blur commits, Escape reverts; **Enter moves focus to the next field** (X to Y, Y to the next row's X; the
  last Y to Tolerance, or Apply while Tolerance is absent) so eight numbers are typed without the mouse
  (P14). Up and Down 0.01 mm, Shift 0.1 mm, each a commit. Invalid text or a value outside -10,000 to
  10,000 mm: 2 px `--field-invalid`, `aria-invalid`, one line under the row "Enter a number from -10000 to
  10000" (12 px, ink), nothing written (P15). An empty field is a valid value (not measured).
- **Undo** (P16, E11): each commit that changes a row is one undo step "Measure mark 1" (typed or taken),
  Clear is "Clear mark 1"; the measured values are laser page data (criterion 19), so they follow undo and
  redo and mark the file changed.
- **A row with one of X and Y** counts as empty (criterion 9): no message (the maker may be typing); the
  header summary and the state line count it as not measured.

### 4.4 Take head position (criterion 8)

| Machine state | Button |
|---|---|
| Profile without a connector address, or the browser build | absent (fields widen) |
| Every state with a position and no controller word (criterion 8, ADR 0081 §8 as amended; a read sends nothing) | enabled |
| Running, Homing, a jog (controller word "Jog") | `aria-disabled` at 40 %, keeps its Tab stop; tooltip line 1 "Take head position", line 2 "Machine is moving" (`aria-description` the same) |
| Door, Check (its position is simulated), Sleep | `aria-disabled` the same way, line 2 "Controller state: Check" (the Home cause of `0118`) |
| No position (no controller, connection lost, the status-read failure) | `aria-disabled` the same way, line 2 "No position from the machine" |
| Reading | `aria-busy`, `LoaderCircle` turning in place of the glyph; the other rows' buttons stay usable |

- The recorded exception to "Hidden, not disabled", for the same reason as Home: a column of buttons that
  comes and goes with the machine state would move Clear under the pointer (P17).
- One press reads the position once (the work position in the machine's coordinates, `0140` criterion 10)
  and fills X and Y in one undo step; the values show in the frame of the answer. No notice (the numbers
  are the feedback). **Focus then moves to the next row's Take head position** (the next mark that is not
  measured; the button keeps focus on the last row), so the loop at the machine is: move the dot, press
  Space, move the dot, press Space (P18).
- Failure: one line under the row, `role="alert"`, 12 px ink with a 14 px `CircleAlert` in
  `--destructive`: "Could not read the head position: Timed out" (the connector's own cause words); gone
  on the next press or edit of that row. The fields keep what they had.
- The button never moves the machine (criterion 20); its name says what it does.

### 4.5 A row whose mark is gone (criterion 10)

A filled row whose mark was deleted on the sheet stays, with its values, so the maker sees what he
measured: the number in muted, the fields read-only text (no field border, muted, still selectable), Clear
present, Take head position absent; one line under it, 12 px muted: "Mark 3 is gone from the sheet". It
does not count. Clear removes the row (it is the only way to remove it); an undo of the deletion on the
sheet turns it into a normal row again.

### 4.6 State line and fit block (criteria 10 to 12)

Under the hairline, in one polite `role="status"` container with `aria-atomic="true"` (P19):

| Measured rows | Content |
|---|---|
| 0 or 1 | 12 px muted "Measure at least 2 marks"; Apply greyed (4.8) |
| 2 or more, all within 1 mm of each other (sheet marks or measured positions) | warn line "Marks too close to fit"; no fit, no numbers; Apply greyed |
| 2 | the fit block, its error line replaced by "2 marks: no error check" (12 px muted) |
| 3 or 4 | the fit block |

**Fit block**, a `dl` on a 64 px label column (review: 88 px wrapped the Error value's last "mm"), 12 px, labels muted, values ink tabular (P20, E7):

| Label | Value |
|---|---|
| Moved | "83.40, 41.20 mm" |
| Turned | "1.80°" (two decimals, the sign of the turn, counter-clockwise positive as the Select bar's rotation) |
| Scale | "100.10 %" |
| Error | "Largest 0.21 mm, RMS 0.15 mm" |

- **OK state** (largest not above the tolerance): the Error value in ink, no glyph.
- **Warn state** (largest above the tolerance, criterion 13): the Error row becomes a warn line in the
  `OutputChecks` look: `TriangleAlert` 16 px ink, "Largest error 0.74 mm; check mark 3" and on a second line
  "RMS 0.41 mm, tolerance 0.50 mm". Never `--destructive`: it is permitted (Apply works). The header summary
  gets the glyph.
- Numbers never read "-0.00"; a scale of exactly 100 % reads "100.00 %".

**Tolerance** field (criterion 13): 0.05 to 5 mm, default 0.50, two decimals, Up and Down 0.05 mm; out of
range: "Between 0.05 and 5 mm" under the field. One undo step "Print and cut tolerance"; saved with the page.

### 4.7 Warn lines (criteria 14, 15; E4, E5)

The `OutputChecks` look, in the status container of 4.6, in this order, each only while it applies:

| Case | Text |
|---|---|
| Scale outside 98 % to 102 % | "Scale 94.20 %: was the sheet printed at 100 %?" (two decimals, the Scale row's number; the spec's "94.2 %" follows) |
| Mirrored (3 or more marks that are not on one line, measured order turns the other way) | "The marks look mirrored: check the numbers" |
| Two measured positions within 1 mm of each other while the fit still has 2 distinct places | "Marks 2 and 4 are measured at the same place" (E4) |

- Collinear marks are no error for this fit (a move, turn and scale needs two distinct points); they only
  switch off the mirror check (E5). The one refusal is "Marks too close to fit" (4.6).
- No warn line repeats the Error row: the tolerance warning is the Error row itself.

### 4.8 Apply and Reset placement (criteria 16, 18)

- **Apply**: enabled while a fit exists and the placement differs from it. One press sets the placed
  sheet's move, turn and scale to the fit, one undo step "Fit sheet to marks"; the canvas shows the sheet
  in its new place in the same frame, no animation, no notice. Focus stays on the button (now greyed).
  The label reads **Apply again** when the sheet was applied once and then moved by hand (criterion 18).
- **Reset placement** (E8; the spec's "Reset"): enabled while a Reset target exists (from an Apply until the
  next Apply or a move of the sheet by hand). One press puts the sheet back exactly, one undo step "Reset
  sheet placement"; the target is then gone. Undo and redo of either step bring the target back with the
  state they restore (P21, the architect stores it with the page).
- Greyed, never absent (both are always in the row, so neither moves): `aria-disabled` at 40 %, Tab stop
  kept, tooltip with the reason: Apply "Measure at least 2 marks", "Marks too close to fit", "The sheet is on
  the fit" (E10); Reset "Nothing to reset". This is the "Armed action" rule for an act that is not possible.
- **Placement line** (row 9), 12 px muted, after an Apply: "On the fit" while the placement equals the fit
  (within 0.001 mm and 0.001°), "Placement differs from the fit" while it does not (criterion 18; a state
  label, not a notice). Absent before the first Apply. The measured values are never cleared by a move.
- Neither button sends anything to the machine; Start and Send in the output group work as before
  (criterion 20, 22).

### 4.9 Other places on the laser page

- **Operations list** (criterion 5): a marks object never becomes a member of an operation. A kind press on a
  selection that holds a placed sheet assigns the sheet as before; the job leaves the marks out. Where the
  `0101` clone member rows list the parts of a placed sheet, the marks object reads "Registration marks (not
  cut)" in muted, no tick, not selectable.
- **Status bar, announcer**: unchanged for the placed page.

## 5. Canvas of the laser page (criteria 5, 6, 17; draft 03)

| What | Look |
|---|---|
| Marks of a placed sheet | Faint crosses: the two crosshair lines only (no circle), true size (8 mm), at least 12 px on screen, 1 device px in `--reg-mark` (`--guide` `#166534` at 60 %, about 2.7:1 on `--canvas-bg`) over a 1 px white casing at 60 %; the number in an 11 px `--guide` tag (100 %, 5.8:1) with the `--dim-halo`, up and right of the centre, screen-space, upright. In the Operations view **and** the Drawn view (P22, E12): a mark never draws as artwork on a laser page. Not hit, not hovered, not in a marquee: a press goes to the sheet below. Not in the job, not exported |
| Measured positions | A 10 px ring, 1.5 px `--accent` over its white casing, with a 2 px centre dot, at each filled row's X and Y; its number in an 11 px `--accent` tag down and left of the ring (the other side from the mark's tag). Drawn while the group exists, in both views, also when the group is folded; never in the job |
| The row in focus | Its mark's cross and its ring drawn at 100 % and 1.5 px; nothing else changes. Lets the maker see which mark the row is |
| Fitted sheet (preview) | While a fit exists and the placement differs from it: the sheet's frame at the fit as the hollow 1.5 px `--preview-new` outline with its casing, dashed 6 on 3 so it never reads as the placed frame; the content is not drawn there ("A preview is never a commit"); a name tag "Fit" (12 px `--accent`) at its top-left. Gone in the frame of Apply |
| After Apply | No preview; the sheet's own frame and content are at the fit, the rings sit on the crosses within the residual |
| Play bar preview and toolpath (`0029`) | The faint crosses as above; no ring, no fit outline; no burn move at a mark (criterion 4) |

- Marks and rings are screen-space overlays drawn after the operation looks, before the selection; the
  200-trace cap of the clone marks does not count them (at most 8 marks and 4 rings per laser page in this
  slice, P22).
- Outside the work area: the existing "Outside the work area" warn line of the output group (criterion 22);
  the fit outline adds nothing.

## 6. Errors and refusals (all texts)

| Case | Where | Text | Role |
|---|---|---|---|
| Delete a mark at 2 marks | canvas notice slot, 8 s | "Registration marks need at least 2 marks." | alert |
| Size or Line width out of range | under the field | "Between 3 and 20 mm", "Between 0.1 and 1 mm" | field invalid |
| Measured value not a number or out of range | under the row | "Enter a number from -10000 to 10000" | field invalid |
| Fewer than 2 measured marks | state line | "Measure at least 2 marks" | status |
| Marks or measured positions within 1 mm | state line, warn look | "Marks too close to fit" | status |
| A measured mark deleted on the sheet | under the row | "Mark 3 is gone from the sheet" | status |
| Two measured positions together | warn line | "Marks 2 and 4 are measured at the same place" | status |
| Largest error above the tolerance | Error row, warn look | "Largest error 0.74 mm; check mark 3" / "RMS 0.41 mm, tolerance 0.50 mm" | status |
| Scale outside 98 to 102 % | warn line | "Scale 94.20 %: was the sheet printed at 100 %?" | status |
| Mirrored | warn line | "The marks look mirrored: check the numbers" | status |
| Head position read failed | under the row | "Could not read the head position: <cause>" | alert |
| Take head position greyed | tooltip line 2 | "Machine is moving", or the `0118` Home cause | description |
| Tolerance out of range | under the field | "Between 0.05 and 5 mm" | field invalid |

No success notice anywhere (Add registration marks, Add mark, Take head position, Apply, Reset placement).

## 7. Keyboard and focus

- Drawing page: `Shift+Ctrl+D`, Tab to **Add registration marks**, Enter. Enter on the selected object
  enters it, Tab and Shift+Tab cycle the marks inside (the `0050` order: number order here), arrows move,
  Delete deletes, Escape leaves.
- Laser page: the Operations tab (its strip key), Tab through the add group, the list, Fixture jobs, then
  the group in this order: header, Sheet button (2+ sheets only), mark 1 X, Y, Take head position, Clear,
  mark 2 ..., Tolerance, Apply, Reset placement, then the Output group. Greyed buttons keep their stop.
- Fast path typed: focus mark 1 X, type, Enter, type, Enter ... eight numbers, then Enter on Apply (4.3).
  Fast path at the machine: focus mark 1's Take head position, Space, move the dot, Space ... (4.4).
- No new shortcut. Escape in a field reverts it and keeps focus; a second Escape returns focus to the
  canvas (the panel rule).

## 8. Accessibility

- Names: the group `section` "Print and cut"; rows `role="group"` "Mark 1"; fields "Mark 1 X", "Mark 1 Y"
  (the unit in `aria-describedby` "millimetres, machine coordinates"); buttons "Take head position for mark
  1", "Clear mark 1", "Apply", "Reset placement"; the Sheet button "Sheet: Coaster".
- Live regions: the state line, fit block, warn lines and placement line are one polite atomic status
  (4.6), updated after a commit, never per keystroke; a read failure is an alert. A greyed button's reason
  is its `aria-description`.
- Colour is never the only cue: warn lines carry the glyph and the words; the faint cross and the ring
  differ by shape (cross, ring with dot) and by the side of their number tags; the fit outline is dashed.
- Contrast: field text 13 px ink on white 10.9:1; muted 12 px 5.0:1 on the panel; the faint cross is
  informational (the number tag and the panel rows carry the same facts), exempt from 3:1 like
  `--axis-guide`; the ring `--accent` 3.9:1 on the canvas with its casing. Hit targets 24 x 28 (36 coarse).
- The canvas is not in the accessibility tree; every fact on it (where the marks are, the fit) is in the
  panel as text.

## 9. Large documents

The fit is computed from at most 4 points on a commit, never while typing; the canvas adds at most 4
crosses, 4 rings and one outline per laser page, drawn without walking the sheet's content. The fit outline
is the frame only, so a sheet with many objects costs nothing extra. Measure the redraw at build against the
`0029` recompute budget.

## 10. Wording (English, shown)

Add registration marks · Registration marks · Mark 1 · Add mark 3 · Size · Line width · Print and cut ·
Sheet · X · Y · Take head position · Clear · Measure at least 2 marks · 2 marks: no error check · Moved ·
Turned · Scale · Error · Largest 0.21 mm, RMS 0.15 mm · Tolerance · Apply · Apply again · Reset placement ·
On the fit · Placement differs from the fit · Fit (canvas tag) · Registration marks (not cut) · Not used ·
the texts of section 6. Header summaries: Not measured · 1 of 4 measured · Error 0.21 mm · Too close.

## 11. Proposals (vetoable at the demo)

| # | Proposal |
|---|---|
| P1 | The entry is a Document tab button "Add registration marks" on a Drawing page, under Fit to content |
| P2 | No shortcut for it |
| P3 | Numbers clockwise from the top-left |
| P4 | A box under 20 mm: marks at its corners, no refusal |
| P5 | The digit up and right of the centre, corner at 3.5, 3.5 mm |
| P6 | The object and its marks only move: no resize or rotate handles, marks always upright |
| P7 | The Style tab shows only the Registration marks block (no Stroke, Fill, Markers) |
| P8 | Delete at 2 marks is refused with an alert |
| P9 | The group sits between Fixture jobs and Output |
| P10 | The group folds; the fold state is per page and session |
| P11 | Several sheets with marks: an inline Sheet picker |
| P12 | Clear is absent on an empty row |
| P13 | X and Y labels above the fields, the unit inside; fallback unit in the labels if values clip |
| P14 | Enter moves to the next field |
| P15 | Measured values from -10,000 to 10,000 mm |
| P16 | Each measure and Clear is an undo step |
| P17 | Take head position is greyed with its cause, not hidden, during a run |
| P18 | After Take head position, focus moves to the next unmeasured row's button |
| P19 | State, fit and warnings are one polite atomic status |
| P20 | The fit block as Moved, Turned, Scale, Error |
| P21 | The Reset target follows undo and redo |
| P22 | Marks on a laser page are faint crosses in `--reg-mark` in both views; measured positions are accent rings; the fit is a dashed preview outline |

## 12. Spec deltas for the product owner

| # | Criterion | Delta |
|---|---|---|
| E1 | "What the maker sees" 1, criterion 1 | "In the Insert group: Registration marks" becomes "In the Document tab of a Drawing page: **Add registration marks**" (no Insert group exists) |
| E2 | 3 | "Delete on the last two removes the object" becomes "Delete on a mark when 2 are left is refused ('Registration marks need at least 2 marks.'); the object is deleted as a whole" |
| E3 | 1 | "10 mm inside the box" puts the marks over the artwork's corners; the PO decides between inside the artwork box (as written), outside it by 10 mm clamped 10 mm inside the page, or the page corners. Notes keep the text as written |
| E4 | 10, 15 | Add: two measured positions within 1 mm give the warn line "Marks 2 and 4 are measured at the same place"; all within 1 mm is "Marks too close to fit" |
| E5 | 15 | "Collinear marks" is not a failure of a similarity fit; the only refusal is "too close" (all marks within 1 mm); collinear marks only switch off the mirror check |
| E6 | 7 | Several placed sheets with marks on one laser page: the group has a Sheet picker; the laser page keeps one sheet reference |
| E7 | 12 | "Moved by X and Y" is the fit's translation of the sheet's origin in machine mm; the PO and the architect confirm or name the reference point |
| E8 | 5, 16 | "Reset" is labelled **Reset placement** |
| E9 | 2 | "The size" is the circle's diameter (3 to 20 mm); the crosshair is the size plus 2 mm, the digit half the size |
| E10 | 16 | Add: Apply is greyed while the sheet is on the fit |
| E11 | 19 | Add: each measured row change and Clear is one undo step |
| E12 | 5 | The faint crosses also replace the marks in the laser page's Drawn view |

## 13. Drafts

`docs/design-drafts/2026-11-print-and-cut/`: `01-marks-on-drawing-page` (the Document tab button, the
object selected and entered, the Style block), `02-print-and-cut-group-states` (not measured and
connected, measuring with a busy and a greyed Take head position, fit OK, warn with scale and gone mark,
too close, typed only without a connector, placement differs), `03-laser-canvas-fit` (faint crosses,
measured rings, the fit preview before Apply and the sheet after Apply). HTML only in this pass; the PNG
beside each is rendered at the next browser pass.
