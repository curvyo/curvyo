# 0078 Font style buttons: UX notes

Owner: ux-engineer, 2026-10-23. For `specification.md` in this folder. Rows of
`docs/design-system.md` are named in quotes. The design-system amendments in §9
go into the build PR.

## 0. Proposed changes to the specification

None blocks the build. Clarifications the implementer follows, and the product
owner may copy into the criteria:

1. **Arrows move focus, they never press** (criteria 1 and 21). A plain
   `radiogroup` checks the item an arrow lands on; here that would load a face
   and write a "Change font" step per arrow press. The weight control uses
   manual activation: arrows move focus, Enter or Space presses (§6).
2. **Glyphs are drawn, not set in a web font** (criterion 1 proposal). The UI
   does not load Source Sans 3 as a CSS font, and loading five faces to draw
   a toolbar is the wrong cost. The five "a" glyphs are SVG paths taken once
   from the Source Sans 3 outlines (OFL 1.1, already recorded) and kept as
   constants in the front end (§2).
3. **Greyed reason with several texts** (criterion 6 gives one-text examples
   only): see §4 for the wording.
4. **Notice with one text left** (criteria 11, 12): "the others have" becomes
   "1 has" when exactly one text was left as it was (§7).
5. **Keys in tooltips only while they work**: "Bold (Ctrl+B)" and
   "Italic (Ctrl+I)" in edit mode, plain names otherwise (§5). Criterion 20
   makes the keys edit-mode only, so a tooltip that names them everywhere would
   name a key that does nothing.

## 1. Placement and size

- **Style group**: `role="group"`, accessible name "Style". Inside it, the
  weight control (five 28 x 28 buttons, 4px apart, 156px), 12px, the Italic
  toggle (28 x 28). Width 196px.
- **Text bar** (Text tool): the bar gets a second group. Font button and Size
  field stay group 1 unchanged; a 1px 20px divider at 25 % and 12px gaps, then
  the Style group. One row needs 537px of overlay row (the 316px bar, 25px of divider
  and gaps, 196px).
- **Select bar Text group**: the Style group is a group of its own directly
  after the Text group (Font, Size), before Object to path, shown under the
  same condition (the selection holds a text) and acting on the texts only.
- **Wrap at 800 x 600, panel open** (overlay row 356px): the Text bar wraps
  by whole groups to two rows, 72px: row 1 Font and Size, row 2 the Style
  group, left-aligned in the pill. The pill stays centred in the overlay row.
  No divider at the start of a row. The Select bar with one text grows from 4
  to 5 rows (180px), the Style group on a row of its own unless it fits beside
  the row before it. Measure both at build and record the numbers in the rows
  "Text bar" and "Select bar Text group". Nothing is cut off; nothing scrolls.
- Fixed widths: the group never changes size with the state; a greyed or
  pressed button is the same box.

## 2. Glyphs

- **Weight buttons**: a lowercase two-storey "a" from Source Sans 3 in the
  weight the button stands for (Extra Light 200, Light 300, Regular 400, Bold
  700, Extra Bold 800), drawn at a 20px em (x-height about 10px), centred
  optically on the x-height, `fill="currentColor"`, `aria-hidden`. 20px and
  not 16px because at 16px the stems of Extra Light and Light are 0.45px and
  0.8px and do not tell apart on a 1x screen; at 20px they are about 0.6px and
  1.0px, Regular 1.7px, Bold 2.8px, Extra Bold 3.4px.
- No letters such as "EL L R B EB": they have to be read and learned; the
  drawn weight is the label. The name is in the tooltip and the accessible
  name.
- **Italic**: Lucide `Italic`, 16px, 1.5px stroke, `currentColor`, like the
  other bar icons. It is a different kind of control (a toggle, not one of
  five), and the 12px gap says so.
- The glyphs do not change with the subject's font: they show the weight, not
  the font. The font is in the Font button.

## 3. States

Colours are existing tokens; nothing new for light or dark.

| State | Look | ARIA |
|---|---|---|
| Idle | Transparent ground, glyph `--toolbar-icon`, `rounded-md` | weight: `role="radio"`, `aria-checked="false"`; Italic: `aria-pressed="false"` |
| Hover | Ground `--editor-accent-hover` | |
| Pressed (current) | Ground `--toolbar-icon-active-bg`, glyph `--toolbar-icon-active-fg`: the look of the arc kind buttons and Link corners | weight: `aria-checked="true"`; Italic: `aria-pressed="true"` |
| Mixed (Italic only) | Ground `--toolbar-icon-active-bg` at 35 %, glyph `--toolbar-icon` | `aria-pressed="mixed"` |
| Mixed (weights) | No weight button pressed (house rule "Mixed-state display": no option highlighted in a segmented control) | all `aria-checked="false"` |
| Thin, Medium, Semibold, Black face | No weight button pressed; the Font button names the face | all `aria-checked="false"` |
| Greyed | Glyph and ground at 40 % opacity, no hover ground, default cursor, focusable, tooltip shown | `aria-disabled="true"`, never `disabled` |
| Focus-visible | 2px `--editor-accent` ring, 1px `--toolbar-bg` offset (as the Flip buttons) | |

- A pressed button is never greyed (criterion 5). Pressed and focused shows
  both the ground and the ring.
- The weight control is `role="radiogroup"`, name "Weight", inside the Style
  group; Italic is a plain toggle button outside it.
- State changes are instant: no transition on ground or opacity.
- Mixed weights show no per-step mark. The Font button says "Mixed" and the
  tooltip of each weight button carries no state line; the maker reads the
  faces in the Fonts tab if he needs them.

## 4. Tooltips

Radix, 400 ms, `side="bottom"`, the bar tooltip surface (`BarTooltip` of
`ArcCurveGroups.tsx`), max 300px. No hint or rule line (no-hint-labels rule).

- Line 1, semibold: the name. "Extra Light", "Light", "Regular", "Bold",
  "Extra Bold", "Italic". In edit mode only: "Bold (Ctrl+B)" and
  "Italic (Ctrl+I)" (Cmd on macOS).
- Line 2, muted, only when it applies: a state or a reason, never advice.
  - Italic mixed: "Mixed".
  - Greyed, one family among the subject texts: "Not in Great Vibes"; for
    an italic text and a weight button: "Not in Source Serif 4 as italic";
    for Italic while it is pressed (no upright counterpart): "Not in
    Lobster as upright".
  - Greyed, several families: "Not in any of the 3 fonts".
  - Missing font with no family to fall back on (criterion 9): "Font missing".

## 5. Pointer and keys

- A press on an available button acts on pointer up (as every bar button).
  A press on a greyed button does nothing and keeps the tooltip open.
- **Focus after a press**: mouse press returns focus to the canvas, or to the
  edit sink with the caret where it was in edit mode (criterion 16); the
  button must not keep focus, so the next typed character goes into the text.
  A key press (Enter, Space) leaves focus on the button, as on the Boolean
  card.
- **Ctrl+B / Ctrl+I** in edit mode: criterion 19. A greyed target does
  nothing and shows nothing; the tooltip line says why when the maker looks.
- The bars stay where they are while a face loads; the pressed state moves to
  the new button at once and the canvas follows when the face is drawn
  (criterion 17).

## 6. Keyboard and focus order

- Tab order in the Text bar: Font button, Size field, weight control (one
  stop), Italic, then out of the bar. In the Select bar the same order inside
  the bar's order: Text group, Style group, Object to path.
- Tab lands on the pressed weight button; with none pressed, on Regular.
- Left and Right (Up and Down too) move focus over all five buttons, greyed
  ones included, without pressing; Home and End go to Extra Light and Extra
  Bold; no wrap at the ends (the rail keyboard model). Enter or Space presses.
- Escape on any of the six returns focus to the canvas, or to the edit sink in
  edit mode, as the Size field does.
- Hit targets 28 x 28 with 4px gaps: the bar's size for a pointer-driven
  desktop tool; no change.

## 7. Notices

Action notice, `role="status"`, 3 s, 4px under the pressed button,
left-aligned to it and kept inside the canvas region; for Ctrl+B and Ctrl+I in
the canvas notice slot. Only when at least one text was left as it was.

- "Bold set on 2 of 3 texts; the others have no Bold."
- With exactly one left: "Bold set on 2 of 3 texts; 1 has no Bold."
- "Italic set on 2 of 3 texts; the others have no italic." /
  "...; 1 has no italic."
- "Upright set on 2 of 3 texts; the others have no upright face." /
  "...; 1 has no upright face."
- Longest case, "Extra Light set on 128 of 140 texts; the others have no Extra
  Light.", is 68 characters. Counts above 999 are not expected; if they occur
  the line may exceed 70 and wraps in the notice's 480px.
- No notice for a greyed press, for a press that changes nothing (the step
  is already pressed), or for one text.

## 8. Large documents and undo

- With 1000 selected texts the pressed, mixed and greyed states come from one
  pass over the selection's face references and the face list, computed once
  per selection or face-list change, not per render or per hover.
- One press, one history step "Change font", whatever the count (criteria 10
  to 12); Undo restores every face in one step, and the buttons show the
  restored state at once.

## 9. Design-system amendments (build PR)

1. "Hidden, not disabled": add the fourth recorded exception, the greyed
   weight and Italic buttons (`0078`, customer request 2026-10-23), with the reason:
   the maker asked to see what his font lacks before the click.
2. "Text bar": two groups, widths of §1, the measured rows at 800 x 600.
3. "Select bar Text group": the Style group after it, measured rows.
4. New row "Style group": §1 to §4 in one row.
5. "Keyboard shortcuts established so far": Ctrl+B and Ctrl+I, edit mode only.
