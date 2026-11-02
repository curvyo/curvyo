# 0069 Tiling: UX notes

Status: Draft for Ready. Owner: ux-engineer. Authority: customer answers of 2026-11-01 (TILE-LIVE B,
TILE-HOME-UI A, TILE-SOURCE A, TILE-SPACING A, TILE-EXTRAS A, TILE-RESULT-GROUP A) in `tiling-plan.md`;
`specification.md` (sketch) for what the options mean. Checked against `docs/design-system.md` (rows
"Select bar layout", "Bar number field", "Action notice", "Segmented icon control", "Trace Advanced block",
`--preview-new`, `--preview-ghost`), `SelectToolbar.tsx` and `specs/0061-align-and-distribute/ux-notes.md`.
**(Proposal)** marks reversible choices, vetoable at the demo. Customer rule 2026-10-16 (YAGNI): no
operating tips or hint labels. Labels that name a control or a state stay; tooltips carry a name and, where
the control is an icon, one rule line. Nothing here says "press Enter" or "Esc to cancel" anywhere on screen.
No dialog, no popover, no dropdown: every choice is a segmented control, a switch or a field.

**Amended 2026-11-02 (customer feedback on PR #247): section 12 wins** over sections 2.1 (items 1, 2, 4), 2.2 (Mirror, Offset: icon names unchanged), 2.3 (circle fields), 2.4 (Variance block, Seed group, Result, count status), 3 (Cancel), 4 (count rows), 5 (circle handles), 6 (tab order), 7 (strings) and 8 (widths). Where this file and section 12 differ, section 12 is right.

## 1. Tile button in the Select bar

- A **text button with a 16 px glyph**, label **Tile**, 28 px high (`BUTTON_CLASS`, `h-7 px-2`), about 64 px
  wide. Glyph in the Command family: a solid-outline 5 px square at the left and two **dashed** squares
  repeating it to the right, 1.5 px absolute stroke, `currentColor`. It must not look like the Grid button's
  2 x 2 block (that one arranges, this one repeats).
- Own `role="group"` "Tiling", placed **after the Grid group (or, when Grid is not shown, after Distribute),
  before the kind groups**, behind the usual 25 % divider. It is in the bar in every selection state of the
  Select tool, so the bar does not jump with the selection (the 0061 rule).
- Tooltip (400 ms, `side="bottom"`): line 1 semibold the name, line 2 `--panel-muted-fg` the rule.

| Selection | Button | Tooltip |
|---|---|---|
| nothing | "Tile", `aria-disabled`, 40 % opacity (the "Remove rounding" look); a press does nothing, silently | "Tile" / "Select an object to repeat." |
| one or more tilable objects (paths, shapes, texts, sketches, groups, images) | "Tile", enabled | "Tile" / "Repeat the selection in a grid or a circle." |
| exactly one tiling | label "Edit tiling" (same slot and glyph), enabled; a second button "Release tiling" follows it, 4 px apart | "Edit tiling" / "Change the grid or circle, the variance and the seed." |
| a tiling plus other objects | "Tile", `aria-disabled` | "Tile" / "A tiling cannot be tiled. Release it first." |

- **Reopen: decided.** Spec criterion 28 makes instances real, selectable children, so a double-click on a
  tiling **enters the group** like on any group (it selects one instance; the spec's "double-click reopens"
  Proposal collides and is dropped). **Edit tiling** opens by its button, or by **Enter on the selected
  tiling** (keyboard form; Enter does nothing else on a selected group). A click selects the tiling as a
  whole. Spec criterion 26/28 wording about double-click should be amended by the PO.
- Release tiling: tooltip "Release tiling" / "Makes a plain group of the instances. The tiling can no longer
  be changed." (Ungroup does the same.) One history step "Release tiling", selection stays on the group, no confirmation (undo is the
  net), no notice. Plain command button, text label, never disabled while it is shown.
- No keyboard shortcut is assumed (as 0061 criterion 13). Tab stop in bar order; Space or Enter press it; a
  mouse press moves focus into the first tiling field (section 2), a key press does too (the bar is replaced,
  so the button is gone).

## 2. The tiling bar (replaces the Select bar's contents)

Pressing Tile / Edit tiling **replaces the whole contents of the Select bar pill** with the tiling bar: the
same pill, same position and ground, no second bar. The tool rail keeps Select highlighted. Entering and
leaving are instant (no motion). Everything is session state until Apply: nothing reaches the document before
it (the preview is never stored).

**Stays the same while the bar is open:** pan, zoom, scroll, page view, Properties panel (read-only for the
source: its fields do nothing visible until Apply; **Proposal**: they stay as they are, not disabled).
**Hand edits:** instances edited by hand before this Apply (criterion 28) are hidden under the preview while
the bar is open and the next Apply overwrites placement, restores deleted cells and rewrites own style only
while opacity or hue variance is on; no warning text is added. **Locked:** canvas selection (clicks and marquee on artwork do nothing), tool switches, page switches,
undo/redo (Ctrl+Z and Ctrl+Y are ignored: a preview is not a history step). Switching tool or page, New,
Open and closing the document **cancel** without a message; the last options are kept as session state for
the next Tile (never saved; a reopened stored tiling shows its own values).

### 2.1 Line 1 and order (left to right; groups wrap whole, 25 px apart, dividers as the Select bar)

1. **Title group**: the word **Tile** (14 px semibold, `--toolbar-icon`, the mode label: the maker always
   sees which mode this is; "Edit tiling" when a stored tiling is open), then **Cancel** and **Apply**.
   Cancel is a plain text button; Apply is the same button with the selected fill (`--toolbar-icon-active-bg`
   ground, `--toolbar-icon-active-fg` text). Both 28 px high, 64 px wide, 4 px apart. They sit first so they
   never wrap away from the maker and never move (the bar is left-aligned so controls keep their place).
   Apply is `aria-disabled` while any field is invalid or the count is refused (section 4); Cancel never is.
2. **Layout** `role="radiogroup"` "Layout": two text segments **Grid** | **Circle** (64 and 72 px, the
   segmented look of the Trace choices; arrows move; one Tab stop). Default Grid. Switching keeps all values
   of both layouts; the fields below change.
3. The layout's **fields** (2.2 or 2.3).
4. **Variation group** (2.4), **Seed group**, **Result group**, **Keep source switch**, **count status**.
   These follow in that order and wrap by whole group, so on a wide window they continue line 1 and on a
   narrow one start the next line.

Field look everywhere: the "Bar number field" (28 px high, white ground, 14 px tabular, right-aligned,
fixed unit suffix inside the right edge with 32 px right padding), the visible label **outside, left**
(Columns, Rows, ...). Counts 44 px without suffix, lengths 72 px `mm`, angles 72 px `°`, percent 72 px `%`.
Typed values reach the preview on every valid keystroke (debounced 120 ms). Invalid text keeps the last
valid preview, marks the field `aria-invalid` (2 px `--field-invalid` ring) and shows the 12 px message chip
under it (the Gap field's chip, `role="status"`).

### 2.2 Grid fields

| Control | Look and name | Rule | Messages |
|---|---|---|---|
| Columns, Rows | 44 px, labels "Columns", "Rows"; defaults 5 and 4 (**Proposal**) | whole number 1 to 100,000; the product is the count | "Enter a whole number", "Must be 1 or more." |
| Spacing | `radiogroup` "Spacing": segments **Gap** \| **Pitch** (52 and 56 px), then **X** and **Y** fields (72 px, `mm`, labels "X" "Y") | Gap: edge to edge of the boxes, 0 = touching, negative allowed (overlap). Pitch: centre to centre, must be more than 0. Switching converts the numbers so the layout does not change | X, Y: "Enter a number"; pitch: "Must be more than 0." |
| Mirror | two 28 px icon toggles `aria-pressed` (Lucide `FlipVertical2`-style row glyph, `FlipHorizontal2`-style column glyph, 16 px, the Link corners toggle's on/off look and 4 px spacing), names "Mirror every second row", "Mirror every second column" | off by default; on = every second row (column) is the source mirrored | none |
| Offset | two 28 px icon toggles "Offset every second row", "Offset every second column" (stagger glyphs: two rows of dots, the lower shifted half a step), mutually exclusive: pressing one releases the other; then the **Shift** field (72 px, `%`, label "Shift", default 50, 0 to 100) | brick / honeycomb; the Shift field is always shown (a muted value with both toggles off does nothing, no hiding) | "Enter a number", "Must be between 0 and 100." |

Gap/Pitch default: Gap, X and Y = 5 mm (**Proposal**). X and Y are always two fields; there is no link toggle.

### 2.3 Circle fields

**Count** (44 px, 1 to 100,000, default 12), **Radius** (72 px, `mm`, more than 0, default half the source's
larger side plus 10 mm), **Start angle** (72 px, `°`, default 0), **Sweep** (72 px, `°`, -360 to 360, default 360),
**Centre X** and **Centre Y** (72 px, `mm`, document coordinates as the Properties position fields), and the
switch **Face centre** (a `ToolbarSwitch`, off by default: instances keep the source's orientation; on: each
turns about its own centre to face the circle's centre). Messages: "Enter a number", "Enter a whole number",
"Must be 1 or more.", "Must be more than 0.", "Must be between -360 and 360." Sweep 360 spaces the instances
over the full circle (count equal parts); less than 360 puts first and last on the ends of the arc. Centre
defaults per the plan: the source's centre shifted left by the radius, so the source is the instance at the
start angle. Centre X, Y, Radius follow the canvas handles live (2.5).

### 2.4 Variation, Seed, Result, Keep source

- **Variance button** (text "Variance" with the 16 px Lucide `SlidersHorizontal`, 28 px, `aria-expanded`,
  controls the block below; the pressed look `--editor-accent-hover` while the block is open, as Trace
  "Advanced"). Never hidden or dimmed. State without colour: a 6 px dot at the glyph's corner when any value is
  non-zero, and the tooltip names it: "Variance: none" or "Variance: rotation, scale". Open or closed is
  session state.
- **Variance block** inside the pill under the last line, after a 25 % divider, at most `min(60vh, 360px)`
  with its own vertical scroll (the Trace Advanced block), rows of 36 px. A table of fields, 64 px wide,
  no suffix inside (the unit is in the row label), headers in 12 px `--panel-muted-fg`:

| Row label (96 px) | Random ± | Per column | Per row |
|---|---|---|---|
| Rotation ° | field | field | field |
| Scale % | field | field | field |
| Offset X mm | field | field | field |
| Offset Y mm | field | field | field |
| Opacity % | field | field | field |
| Colour ° | field | field | field |

  **Random ±** is the range around the layout value (a value v gives a draw in -v to +v from the seed).
  **Per column** and **Per row** are steps added per column or row index (stepped variance); in the Circle
  layout the two columns are replaced by one, **Per step** (added per instance along the arc). Defaults all
  0. Scale is uniform (per-axis scale is out, a later question). Opacity is percentage points of the
  source's opacity, clamped 0 to 100 on the result. Colour is a hue shift in degrees of the source's fill and
  stroke (clones receive it as their own style, copies write it, the `0057` mechanism). Messages: "Enter a
  number"; Random ± also "Must be 0 or more."; steps may be negative. Tab goes across a row, then down.
- **Seed group**: label "Seed", a 112 px field (whole number, 0 up to the generator's maximum, default a
  fresh seed drawn at entering, never 0 by surprise), then the dice: a 28 px icon button (Lucide `Dices`, 16
  px), name and tooltip "New random seed". The dice sets a new seed and updates the field and preview; the
  same seed always shows the same layout. Never disabled (with no random value it simply changes nothing
  visible). Messages: "Enter a whole number", "Must be 0 or more."
- **Result group**: `radiogroup` "Result", label outside "Result", segments **Clones** | **Copies** (64 and
  64 px). Default Clones (TILE-RESULT). Only **images** cannot be cloned: they become copies in either
  setting, and the status says so (section 4). Sketches tile as clones.
- **Keep source in place**: a `ToolbarSwitch`, label "Keep source in place" (short label "Keep source"),
  **default on** (criterion 22). Tooltip "Keep source in place" / "On: the source becomes the first cell, inside
  the group. Off: the source stays untouched and the group sits above it." On: the source is not drawn as a
  preview instance (it is the cell). Off: all cells including the first are previewed over the source.
- **Count status**: 12 px `--toolbar-icon`, tabular, "48 instances" ("1 instance"), always shown, last on
  the line. It is the count that will be made (the source counted when it is an instance).

## 3. Apply, Cancel, Enter, Escape

- **Apply** (button, or Enter anywhere in the bar or on the canvas): one history step, labelled **Tile** (new)
  or **Edit tiling** (a stored tiling reopened). The result is one new group named **Tiling** in the source's
  layer, selected; the maker is back in the Select bar with the tiling selected. Focus returns to the canvas.
  Enter in a field applies the whole tiling (the field's valid text is already in the preview); with an invalid
  field Enter writes nothing and keeps the chip.
- **Cancel** (button, or Escape anywhere, also inside a field, also with the variance block open): drops the
  preview, writes nothing, restores the Select bar and the previous selection. No cascade, no confirmation.
- A press on a button with the mouse returns focus to the canvas; a key press stays (the Flip rule), except
  Apply and Cancel which always return it.
- Applying an unchanged reopened tiling is silent and writes no history step.

## 4. Notices

All in the bar, never a toast and never modal. Chip surface: 12 px, `--toolbar-bg` ground,
`--panel-elevation-shadow`, 8 px radius, 4 px under its anchor, max 320 px wide, shifted by `useNoticeShift`.

| When | Where and role | Text |
|---|---|---|
| Count above 10,000 and up to 100,000 | inline after the count status, `CircleAlert` 12 px, `role="status"` | "Over 10,000 instances. The preview is simplified." |
| Count above 100,000 | under the count status, `role="alert"`, refusal look (`ring-1 --field-invalid`, text `--field-invalid`); Apply `aria-disabled`; the preview shows only the source | "Too many instances. 100,000 at most." |
| Clones chosen, source holds an image | inline after the count status, `role="status"` | "Images become copies." |
| Apply refused by the core (the commit's own refusal, before the first write) | under Apply, `role="alert"`, refusal look, until the next change | the core's one-sentence reason, e.g. "Could not tile: the source is empty." |
| Field errors | the field's chip (2.1) | the messages of section 2 |

A refusal never changes the options; the maker fixes the value and presses Apply again. The count warning
never blocks Apply.

## 5. Canvas

- **Instances** are hollow outlines in `--preview-new` (1 px, screen width at every zoom); under each a
  `--preview-ghost` fill with the instance's own variance (opacity, hue), so colour and opacity variance show
  live. The **source** stays drawn in its own committed style (the "old" half of blue-new/black-old). With
  Keep source on, the source is cell (0, 0) and only the other cells are drawn; off, the first cell is drawn
  over the untouched source. On a reopened tiling the existing instances are hidden while the preview shows.
- **Large counts:** only instances meeting the viewport are drawn. Above 2,000 visible instances the ghost
  fill is dropped and above 10,000 instances each is drawn as its 1 px box (the warning says "simplified");
  the preview never takes longer than one frame at 10,000 (CPU-time test, architect).
- **Circle handles:** a **centre handle**, a 12 px ring with a cross, 2 px `--accent` over a 1 px white
  casing, `move` cursor, drag moves Centre X and Y (snapping like other handles, `0060`, Ctrl suspends), and
  a **radius handle**, a 10 px square on the circle at the start angle, 2 px `--accent`, resize cursor of the
  radial direction, drag changes Radius. A thin dashed circle or arc in `--preview-new` at 50 % shows the
  path. Both handles have a 12 px hit area and keep it at small zoom. A live readout chip at the pointer, the
  "Live transform readout": "Centre 120.0, 80.0 mm" or "r 42.0 mm" (one decimal, real minus sign). Grid has
  no handles: the fields are enough in this version.
- **Draw order, bottom to top:** artwork, instance ghost fills, instance outlines, circle path, source
  selection box, handles, then the DOM chip. The Linked clone marks (`0049`) are off while the bar is open, and
  for a selected tiling as a whole (no 200 dashed traces); it shows the group box and the tag "Tiling" in the
  clone-tag chip style (`aria-hidden`). With one instance selected inside the entered group the usual clone
  marks of that clone show.
- **A stored tiling** is an ordinary, expandable group row named "Tiling" in the Layers tab; its instances
  are listed and selectable there and by entering the group on the canvas. Properties shows the group's
  position and size. Hand edits last until the next Apply (criterion 28); to keep them, Release tiling.
- Pages are ignored: instances land where the layout puts them, off the page too.
- **Live source edits (amended 2026-11-02, customer on PR #247: "it should be live"):** while the source of a
  stored tiling is scaled with the Select handles, or a node, handle or side of it is dragged with the Node tool,
  every clone instance is drawn as itself with the edit in its own cell in the same frame, as the release leaves
  it; a style preview reaches the instances that take the source's style. **(Proposal)** the instances leave no
  drag ghost and are drawn as themselves in a Node drag too (not as the blue clone outline of `0049`): 10,000
  ghosts or blue outlines are noise and cost about 100 ms a frame.

## 6. Keyboard and accessibility

- Tab order: Cancel, Apply, Grid/Circle (one stop, arrows move), then the fields in the order of section 2,
  Variance, (block fields row by row, only while open), Seed, dice, Result, Keep source in place. Shift+Tab
  reverses. After Apply or Cancel the focus is the canvas.
- Entering the mode puts focus on the first field (Columns or Count) with its text selected; a screen reader
  hears "Tile, Columns". Hits 28 px (36 px with a coarse pointer), focus-visible ring 2 px `--editor-accent`
  with 1 px `--toolbar-bg` offset, as every bar control.
- Names: groups "Layout", "Spacing", "Result" are `radiogroup`s with `aria-label`; icon toggles have
  `aria-pressed` and a name; fields have a visible label; units are in the accessible name ("X, millimetres",
  "Start angle, degrees", "Shift, percent").
- A polite live region says "Tiling, 48 instances" on entering and after a count change (debounced 500 ms),
  and the refusals use `role="alert"`. Preview, handles, ghosts are `aria-hidden`.
- State never rests on colour alone: the dot on Variance has the tooltip, toggles change glyph fill,
  segments have the selected fill plus `aria-checked`. Contrast as the existing bar tokens (no new colour).
  Light and dark theme use the existing tokens; **no new token**.
- Reduced motion: nothing animates.

## 7. Exact strings

Buttons and labels: Tile, Edit tiling, Release tiling, Cancel, Apply, Grid, Circle, Columns, Rows, Gap,
Pitch, X, Y, Shift, Count, Radius, Start angle, Sweep, Centre X, Centre Y, Face centre, Variance, Rotation, Scale,
Offset X, Offset Y, Opacity, Colour, Random ±, Per column, Per row, Per step, Seed, Result, Clones, Copies,
Keep source in place. Accessible names: Mirror every second row, Mirror every second column, Offset every
second row, Offset every second column, New random seed. Group name of the result: **Tiling**. History
steps: **Tile**, **Edit tiling**, **Release tiling**. Status: "{n} instances", "1 instance", "Over 10,000
instances. The preview is simplified.", "Too many instances. 100,000 at most.", "Images become copies." Tooltips: section 1 (Tile, Edit tiling, Release tiling) and 2.4 (Keep source, Variance state, New
random seed). Field errors: Enter a number, Enter a whole number, Must be 0 or more., Must be 1 or more.,
Must be more than 0., Must be between 0 and 100., Must be between -360 and 360. (Sentence case, no
exclamation marks, no "please".)

## 8. How the bar fits the width

- Wrapping is the bar's own (`BAR_ROW_CLASS`): whole groups, 25 px apart, dividers hidden at a row start
  (`useRowStarts`). No horizontal scroll, no overflow menu, nothing hides.
- Rough group widths: title group 170, Layout 140, Columns + Rows 200, Spacing about 330, Mirror 60, Offset 170,
  Variance 110, Seed 190, Result 150, Keep source 130, count 90. Circle: Count 100, Radius 130, Start 130,
  Sweep 130, Centre X and Y 2 x 110, Face centre 110. Expect **two lines** for Grid at a 1440 px window (about
  1000 px area), three at 1100 px, four at 800 x 600 (the area is then about 356 px: the widest group, Spacing,
  must wrap its X and Y below the pill instead of leaving the pill, the 0061 review item 3 lesson).
- Apply and Cancel are on line 1 in every width, because they come first.
- The variance block adds its own rows below the last line; it never pushes Apply (which is above).
- A group never exceeds the area's width: minimum content width of the bar is the widest group, 260 px.

## 9. UI review checklist

1. Select bar: Tile / Edit tiling / Release tiling in the states of the table in section 1; disabled look is
   40 % opacity, silent press; tooltip text exact.
2. Entering replaces the bar, no other bar or layer appears; the title group shows "Tile" / "Edit tiling".
3. Preview is live on every valid keystroke and on dice; instances are `--preview-new`; the source is not
   changed; nothing in History before Apply.
4. Enter in a field applies once; Escape in a field cancels; an invalid field blocks Enter and Apply with its
   chip; the last valid preview stays.
5. Result group named "Tiling" in the source's layer; one history step "Tile" (or "Edit tiling"); undo
   removes it and nothing else; reopen shows the stored values and seed; the same seed gives the same layout
   after save and open.
6. Gap/Pitch switch converts without moving the layout; offset toggles are exclusive; mirror toggles
   independent; Circle handles drag and match the fields; Sweep under 360 puts first and last on the arc ends.
7. Variance: the dot and tooltip state; the block scrolls inside 360 px; Tab order row by row; defaults 0.
8. Counts: 10,001 shows the status warning and a simplified preview; 100,001 shows the refusal and Apply is
   disabled; 10,000 previews within one frame.
9. Keyboard only, all of it: reach every control, apply, cancel, release; focus after Apply is the canvas;
   focus-visible ring everywhere; coarse pointer 36 px.
10. Widths 1440, 1100, 800 x 600 in both layouts: nothing outside the pill, no lone trailing control, Apply
    and Cancel on line 1, dividers right.
11. Light and dark theme contrast of the new text and handles (4.5:1 text, 3:1 graphics); no new token.
12. Locked state: clicking artwork, switching tool or page and Ctrl+Z behave as in section 2 and never leave a
    stuck preview.
13. Screen reader: live region, names with units, `aria-hidden` on canvas overlays.
14. No operating tips or hint text anywhere (customer rule 2026-10-16).

## 10. Questions for the lead (defaults given)

- Scale per axis (sketch criterion 3 says "uniform or per axis"): default **uniform only** in this version.
- Grid canvas handles (drag the pitch or the corner): default **none**, fields only.
- Reopen by Enter on the selected tiling (double-click enters the group, section 1): default **yes**.
- Keyboard shortcut for Tile: default **none**.

## 11. UX review (2026-11-02, ux-engineer)

Verdict: **CHANGES REQUESTED** (two behaviour findings, one canvas finding; all else matches sections 1 to 8).
Browser-checked at 1440 x 1000, 1100 x 800 and 800 x 600 (Chromium, dev build). Screenshots:
`/tmp/claude-1000/-home-marc-curvyo/a768e45d-9f28-55af-89f0-ec536eb460a1/scratchpad/ux0069/`.

Passes: Tile button states (nothing: `aria-disabled`, 40 %, silent press; one or several: enabled; tiling:
Edit tiling + Release tiling); Enter on the Edit tiling button reopens with identical values and seed; dice
changes the seed; Apply returns focus to the canvas; group "Tiling" in Layers; history steps Tile, Edit
tiling, Release tiling; Gap/Pitch converts (5 mm gap = 26 mm pitch for a 21 mm box); offset toggles are
exclusive, mirror toggles independent; Variance dot and tooltip state ("Variance: opacity, colour"); Circle
shows Per step; centre and radius handles drag, the readout chip shows "Centre x, y mm" and "r x mm";
Escape in a field cancels; Ctrl+Z and clicks on artwork are ignored while the bar is open; tool switch
cancels; 10,001+ shows the warning, 100,001+ the alert; no overflow at 1100 or 800 px (X and Y wrap in the
pill), Apply and Cancel stay on line 1; no hint text.

Fixed in this review (visual only):
- Field and alert chips were as narrow as their anchor and wrapped to four lines ("Enter a whole number",
  "Too many instances. 100,000 at most."); chips now size to their text (max 320 px).
- Apply showed no disabled look above 100,000 instances; it now takes the 40 % opacity of the other
  disabled buttons.

Open findings:
1. Apply is not `aria-disabled` while a field is invalid (Columns "abc": Apply reports false, section 2.1
   requires true; Enter is blocked correctly). Fix: the bar knows the invalid fields (lift the field error
   into the session, or reuse the `aria-invalid` check of the Enter path), set `aria-disabled` and the 40 %
   look, and ignore the press.
   Resolved (2026-11-02): Apply takes `aria-disabled` and the 40 % look while any field is invalid (`frontend/tests/tilingBar.test.ts`); Enter is blocked likewise.
2. While the bar is open the source keeps its full transform handles (rotate, scale, skew arrows, corner
   radius dots). They collide with the circle centre and radius handles (screenshot `13-circle2.png`,
   `15-drag-centre.png`) and suggest edits that do nothing. Fix (editor-wasm overlay): draw only the
   dashed selection box while the tiling bar is open (section 5 says "source selection box").
   Resolved (2026-11-02): with the bar open only the dashed selection box is drawn and the Select cursor never shows a transform handle (`no_transform_handle_is_hit_while_the_bar_is_open`); presses already picked circle handles only.
3. Minor: the Columns/Count focus ring touches its outside label at 44 px width; add 4 px gap between
   label and field.
   Resolved (2026-11-02): the label gap is 8 px (`gap-2`), as the other bar fields.
4. Not exercised: dark theme, coarse pointer (36 px hits), the box-only preview above 10,000 visible
   instances at low zoom, screen reader output. Tester or the demo should cover them.

## 12. Amendment 2026-11-02: the bar redesigned after the customer's feedback on PR #247

Authority: customer feedback of 2026-11-02 (two screenshots, grid and circle with Variance open, translated in the
lead's brief). **(Proposal)** marks my choices, vetoable at the demo. Customer rule 2026-10-16 still holds: no
operating tips or hint labels. Nothing on screen says "double-click" or "Esc"; the handles, the Escape key and the
entry boxes are learned from the other tools (consistency replaces explanation). Labels and tooltips only name.

### 12.1 What the customer asked for, and what it becomes

| Feedback | Result |
|---|---|
| Cancel goes, Apply far right | No Cancel button; Escape cancels. Apply is pinned at the right end of line 1 |
| Grid / Circle: only symbols | Two 28 px icon segments, names in tooltip and `aria-label` |
| Centre X/Y out of the bar | Centre handle; double-click opens an entry box (X, Y) |
| Radius the same | Radius handle; double-click opens an entry box (r) |
| Start angle and sweep: handles, double-click, out of the bar | Two new handles with entry boxes (Start, Sweep) |
| Seed goes completely | No seed control or display; the stored seed stays |
| Instance count goes | No running count; a quiet notice icon only when a limit is hit (12.3) |
| Keep source vertically centred | One 28 px centre line for every control on line 1 (12.2) |
| Variance more compact | Three blocks of two channels (amended 2026-11-02), narrow fields, 24 px rows (12.5) |

The Count field of the circle stays: it is an input, not the count display.

### 12.2 The bar, final order (left to right)

Line 1 is a left cluster that wraps by whole group (25 px apart, dividers as in section 8) and **Apply**, a
separate right column that never wraps away and never moves.

1. **Title**: the word **Tile** ("Edit tiling" when a stored tiling is open), 14 px semibold `--toolbar-icon`.
2. **Layout**: `radiogroup` "Layout", two **28 x 28 px icon segments** in the Offset chip's pill look (1.5 px
   `--toolbar-icon` ring, selected = `--toolbar-icon-active-bg` / `-fg`). Glyphs, 16 px, 1.5 px stroke, `currentColor`,
   custom like `StaggerGlyph`: **Grid** = 3 x 3 squares of 3 px; **Circle** = 6 squares of 3 px on a ring. Tooltip
   (400 ms) is the name only: "Grid", "Circle". Roving tabindex, one Tab stop, arrows choose. Default Grid; values of
   both layouts are kept.
3. **Mode fields.** Grid: Columns, Rows, Spacing (Gap | Pitch, X, Y) as in 2.2. Circle: **Count** (44 px) and the
   switch **Face centre**. Nothing else: centre, radius, start and sweep are on the canvas (12.4).
4. **Mirror and Offset toggles** (Grid only): names, glyphs, exclusivity and the Shift field as in 2.2.
5. **Variance** button (as 2.4: glyph, dot, tooltip "Variance: none" or "Variance: rotation, scale", `aria-expanded`).
6. **Result**: label outside "Result", segments **Clones** | **Copies** (as 2.4).
7. **Keep source**: the `ToolbarSwitch`, label **Keep source in place** (tooltip as 2.4). **Vertical centring:**
   the group is `h-7 inline-flex items-center gap-2`; the switch track and the 20 px label line share the 28 px
   centre line. Review test: the centre y of Keep source equals that of Title, Layout and the fields within 0.5 px
   at 1440, 1100 and 800 px (the same row test applies to every other control of the row).
8. **Notice slot** (12.3): a 16 px icon, only while a notice is active.
9. **Apply** (far right): 64 x 28 px, selected fill, as now. Same rules as before: `aria-disabled` and 40 % opacity
   while a field is invalid or the count is refused. In the DOM and the Tab order it comes last.

Layout in CSS terms: `flex`, the cluster `min-w-0 flex-1 flex-wrap`, Apply `shrink-0 self-start`, so on a wrapped
bar Apply stays at the top right of line 1. The Variance block opens below both, full width, as before.

**Escape cancels** the tiling anywhere in the bar and on the canvas, also inside a bar field and with the Variance
block open, **except** with a handle entry box open: then Escape closes only the box (12.4) and the next Escape
cancels. Enter applies as before (in a bar field, on the canvas); with a box open, Enter belongs to the box.
Focus on entering: Columns or Count, text selected. Focus after Apply or Escape: the canvas.

### 12.3 What remains of the count (decided)

No count text. One **notice slot** (a 16 px `CircleAlert`, focusable, 28 px hit area, `role="status"` with the
text as its accessible name) after Keep source, shown only when a notice is active; its tooltip lists the active
lines, one per line:

| When | Look | Text |
|---|---|---|
| Over 10,000 and up to 100,000 instances | icon, `--toolbar-icon` | "Over 10,000 instances. The preview is simplified." |
| Clones chosen and the source holds an image | same icon | "Images become copies." |
| Over 100,000 instances | icon in `--field-invalid`, **and** the chip under it stays visible (`role="alert"`, refusal look, as section 4); Apply disabled | "Too many instances. 100,000 at most." |

Reason: a refusal that blocks Apply must be readable without a hover; the two soft notices are quiet. The sr-only
live region keeps "Tiling, 48 instances" on entering and when a limit state changes; it no longer announces every
count change. The Apply refusal under Apply (section 4) is unchanged.

### 12.4 Circle handles and their entry boxes

Only in Circle; Grid has none. Drawn above the preview and the dashed selection box; the source shows nothing else.
**Path:** the swept arc dashed 1 px `--preview-new` at 50 % (as now); the unswept rest of the circle dotted 1 px at
20 %; when |Sweep| is under 360, two 1 px 50 % lines from the centre to the start and sweep knobs. All `aria-hidden`.

| Handle | Where | Look | Cursor | Drag |
|---|---|---|---|---|
| Centre | the circle's centre | 12 px ring with cross, 2 px `--accent` over 1 px white casing (as built) | `move` | moves Centre X, Y |
| Radius | on the circle at the **middle of the arc** (start + sweep / 2; at a full circle opposite the start), **moved from the start angle** so it no longer sits on the source | 10 px square, 2 px `--accent` over white casing (as built) | the stock resize cursor nearest the radial direction | Radius = distance to the pointer, more than 0 |
| Start | on the circle at the start angle | the **Parameter handle** (10 px round, idle white ground, hover `--accent-hover`, dragging or box open solid `--accent`) | `pointer` | rotates the whole arrangement; Sweep is kept, so the end moves with it |
| Sweep | on the circle at start + Sweep | the same Parameter handle | `pointer` | changes Sweep, start fixed |

- **Sweep rest position:** at |Sweep| = 360 the sweep knob is drawn 14 px of arc before the start knob (the arc
  handles' 14 px rule), so the two never lie on each other. Dragging it tracks the pointer angle relative to the
  start, unwrapped continuously, clamped to -360 to 360; within 5 px of the full turn it snaps to exactly 360.
- **Hit size:** 12 px pick distance around each handle's centre, at every zoom (as built). Nearest wins; on a tie the
  order is Sweep, Start, Radius, Centre. Below a 20 px on-screen radius the handles overlap; the maker zooms in or
  types (the keys below).
- **Angles:** counterclockwise as seen on screen, 0 pointing right (criterion 11). Start is shown from -180 to 180°
  and stored as typed (normalised to that range). Free drag: 0.1°; **Ctrl** snaps to the 15° and 22.5° stops (the arc
  handles' rule; the Select tool's handles use Ctrl to snap angles too). No other key.
- **Snapping, centre and radius:** as the other Select-tool handles (`0060`, `0090`), Ctrl suspends; no tiling-only
  key. The radius uses the snapped pointer point.
- **Readout chip** at the pointer, the "Live transform readout", one decimal, real minus sign:
  "Centre 120.0, 80.0 mm" / "r 42.0 mm" / "start 30.0° · sweep 120.0°" (start drag) / "end 150.0° · sweep 120.0°"
  (sweep drag).
- **Hint chips:** none.

**Entry box** (opened by a **double-click on a handle**): the "Transform entry chip" surface, built with
`EntryField` and the placement and message card of `MoveEntryChip` (`frontend/src/components/`; patterns of `0052`
and `0072`: `PenEntryChip`, `PlacementChip`). New component `TilingEntryChip.tsx`; the handle it belongs to stays
in its dragging look while the box is open.

| Handle | Fields (28 px high, fixed unit suffix, 14 px tabular) | Accessible names |
|---|---|---|
| Centre | "X" and "Y", 100 px each, 4 px apart, `mm`, 20 px left padding | "Centre X, millimetres", "Centre Y, millimetres" |
| Radius | "r", 100 px, `mm` | "Radius, millimetres" |
| Start | "Start", 126 px, `°`, 44 px left padding | "Start angle, degrees" |
| Sweep | "Sweep", 126 px, `°`, 44 px left padding | "Sweep, degrees" |

- **Prefill:** the current value (up to two decimals, centre in document coordinates as Properties shows). Focus on
  the first field, text selected; the double-clicked handle decides the box, so Centre opens on X.
- **Placement:** Radius, Start, Sweep: outward of the handle (centre-to-handle direction), 10 px clear of its glyph,
  upright, flipped and clamped with the readout rule, right of `--rail-right` + 12; Centre: left edge 16 px right of
  the glyph, top 16 px below it (the Move chip rule). Follows pan, zoom and resize.
- **Typing previews** on every valid keystroke (debounced 120 ms), like the bar fields and the `0052` chip (a
  deliberate difference from the numeric entry chip of the Select tool, which does not preview).
- **Keys:** Tab and Shift+Tab loop through the box's fields (Centre: X, Y). **Enter** keeps the typed values, closes
  the box and returns focus to the canvas; it does not apply the tiling (the next Enter does). Enter on unedited text
  closes the box and changes nothing. **Escape** closes the box and **restores the values it opened with**; it does
  not cancel the tiling (the next Escape does). No arrow stepping, no modifiers.
- **A press elsewhere** (not swallowed; a press on another handle starts its drag) and a tool switch, page switch or
  window blur close the box as Escape does.
- **Errors:** a refused value keeps the box open, re-selects the text, marks the field `aria-invalid` with the 2 px
  `--field-invalid` ring and shows the Move chip's message card (12 px, `CircleAlert`, `role="status"`): "Enter a
  number" (all), "Must be more than 0." (Radius), "Must be between -360 and 360." (Sweep). The error clears on the
  next keystroke. While invalid, the last valid preview stays and Enter keeps the box open.
- **Keyboard route (Proposal):** handles are canvas pixels, so the bar's old Tab path to these values is gone. With
  focus on the canvas and the circle bar open, **C**, **R**, **A** and **S** open the box of the Centre, Radius,
  Start angle and Sweep handle, placed as if double-clicked, also when the handle is off screen or lies on another.
  They are free in this mode (selection and tools are locked). They are named in the keyboard map in
  `docs/design-system.md` and the shortcut list, never on the canvas. If the customer prefers no keys, the values
  are then reachable by pointer only; I do not recommend that.

### 12.5 Compact Variance block

**Amended 2026-11-02** (customer on PR #247: "color und opacity können ruhig auch noch daneben, das spart eine
zeile"): **three blocks of two channels** instead of two blocks of three, so the block has two rows.

Opens under the bar (full width, 25 % divider above, as before). **Three blocks side by side, two channels each**;
each block has its own header row once, then two rows. Row height **24 px**, row gap 4 px, fields **24 px high**,
14 px tabular, no suffix inside (units in the row labels), 6 px inner padding; header row 20 px, 12 px
`--panel-muted-fg`, right-aligned over its column. Block padding 8 px top and bottom, 8 px left and right.

| Block (left) | Random ± | Per column | Per row |
|---|---|---|---|
| Rotation ° | field | field | field |
| Scale % | field | field | field |

| Block (middle) | Random ± | Per column | Per row |
|---|---|---|---|
| Offset X mm | field | field | field |
| Offset Y mm | field | field | field |

| Block (right) | Random ± | Per column | Per row |
|---|---|---|---|
| Colour ° | field | field | field |
| Opacity % | field | field | field |

- **Widths:** label 56 / 68 / 60 px + 56 + 64 + 56 px columns + 3 gaps of 4 = 244 / 256 / 248 px; two gutters of
  12 px and padding 8 + 8: **788 px**. "Per column" gets a 64 px column so no header is clipped.
- **Height:** 8 + 20 + 2 x 24 + 4 + 8 = **88 px** (116 px before).
- **Fewer columns on a narrower bar:** the blocks wrap (flex-wrap, 12 px gutter, 8 px between lines): under about
  788 px of bar two blocks per line and the third below with its own header row; from 500 px down see below.
- **Circle:** the middle and right columns collapse to one, **Per step**; each block is label + 2 x 56 + 4, three
  blocks fit from about 560 px.
- **Narrow areas (under 500 px, e.g. 800 x 600, area about 356 px):** the blocks stack into one column of six
  rows under **one** header row (Random ±, Per column, Per row), label width 72, 6 x 24 + 5 x 4 + 20 + 16 = 200 px
  high, 268 px wide. Container query on the block, not a media query.
- **Coarse pointer:** fields 36 px, rows 36 px, one column.
- **Tab order:** left block row by row (across, then down), then the middle block, then the right block. Names:
  "Rotation random, degrees", "Rotation per column, degrees" ... (`{channel} {column}, {unit}`), as built.
- Defaults, ranges, messages and the active-channel dot of the button: unchanged (2.4).

### 12.6 Seed

No control, no display, no dice. A new Tile draws a fresh seed (platform random) and stores it; Edit tiling uses the
stored seed, so the layout never changes by itself. **Reroll: none** (default, the customer said "completely");
a maker who wants another random look undoes the Tile and tiles again. Open question 2 below if the customer
misses it.

### 12.7 Strings

**Removed from the bar:** Cancel, Seed, New random seed, Centre X, Centre Y, Radius, Start angle, Sweep (as bar
labels), "{n} instances", "1 instance". **New:** tooltips and names "Grid", "Circle" (icon segments; group name
"Layout" stays); entry box labels "X", "Y", "r", "Start", "Sweep" with the names of 12.4; readouts "start {a}° ·
sweep {b}°", "end {a}° · sweep {b}°". **Kept:** Tile, Edit tiling, Release tiling, Apply, Columns, Rows, Gap,
Pitch, X, Y, Shift, Count, Face centre, Variance and its row and column names, Result, Clones, Copies, Keep source
in place, the three notices of 12.3, the field messages. Group name **Tiling** and the history steps are unchanged.

### 12.8 Review checklist (replaces items 3, 4, 6 to 8, 10 and 13 where they differ)

1. No Cancel, Seed, dice, count text or circle fields (except Count, Face centre) in the bar; Escape cancels from
   a bar field, the canvas and with the Variance block open; Apply is at the right end of line 1 at 1440, 1100,
   800 x 600 and does not move when the bar wraps.
2. Keep source and every control of line 1 share one centre line within 0.5 px.
3. Layout icons: names, tooltips, arrows, one Tab stop, contrast 3:1 for the glyphs, both themes.
4. Four handles: look, 12 px pick, tie order, cursors, readouts; Radius no longer on the start angle; sweep knob
   14 px before the start at 360; snap to 360; Ctrl stops; sweep never flips at a full turn.
5. Each double-click opens the right box at the right place; Enter keeps and closes; Escape restores and does not
   cancel the tiling; the next Escape does; invalid text keeps the box and the last preview; focus returns to the
   canvas; keys C, R, A, S open the same boxes.
6. Variance: 116 px high at 520 px wide, no clipping of headers, Circle with Per step, 1 column under 500 px, Tab order.
7. Notice slot: 10,001 icon and tooltip; 100,001 refusal text and Apply disabled; image notice; no number on screen.
8. No operating tips or hint text anywhere.

### 12.9 Questions for the lead (defaults given)

1. Keys **C, R, A, S** open the handle boxes from the keyboard: default **yes** (accessibility; no on-canvas hint).
2. Reroll control for the hidden seed: default **none**. If wanted later: a 28 px `Dices` button named "New random
   seed" in the Variance block header, shown only while a Random ± value is not zero.
3. Enter in a handle box keeps the value and closes the box; the next Enter applies the tiling: default **yes**
   (the alternative, Enter applies at once, would skip the check of the other handles).
4. Radius handle moved to the arc's middle: default **yes**, because the start angle now has its own handle.
