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
  the selection. Hidden only when the page holds nothing to act on (align:
  no object on the page; distribute: fewer than two).
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
- Distribute gaps: three vertical bars (3 wide) with equal gaps; the outer
  two solid, the middle one outlined (it is the one that moves).
- Distribute centres: the same three bars, each with a 1.5 px dot at its
  centre on one horizontal dashed guide — the centres are evenly spaced.
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

Distributing on the page spreads over the **page's whole extent**: the first
box starts on the page's start edge, the last ends on its end edge (equal
gaps), or the outermost centres sit on those edges (equal centres) — the way
Illustrator's distribute-to-artboard works. A typed gap spaces from the page's
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
