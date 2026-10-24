# 0084 Text bar second row: UX notes

Owner: ux-engineer, 2026-10-24. For `specification.md` in this folder. Rows of
`docs/design-system.md` are named in quotes. Widths come from the code on
`story/text-vertical` (`TextBar.tsx`, `SelectToolbar.tsx`): pill padding 8px
each side, 12px + 1px divider + 12px = 25px between groups, 4px inside a group.

## 0. Proposed changes to the specification

1. **Glyphs moves to row 2 (criteria 6, 7).** With Glyphs and On path on row 1
   the second line at 800 x 600 needs 196 + 25 + 28 + 25 + 28 + 25 + 28 =
   355px in a 308px inner width, so the expand button wraps to a third line and
   criteria 18 and 20 fail. Row 1 becomes: Font, Size, Alignment, Style, On
   path, expand. Row 2: Writing mode, Glyph orientation, Taper, **Glyphs**,
   then the On path options of `0080`. Glyphs opens a pane and sets no value,
   so it is rare in the sense of this story. If the customer keeps Glyphs on
   row 1, the bar is 3 lines closed at 800 x 600 once On path ships (2 until
   then).
2. **Font and Size are 4px apart (criterion 6, "one group").** At the 12px of
   today, 28 + 12 + 120 + 25 + 124 + 16 = 325px, one pixel over the 324px
   overlay row. At 4px (the gap inside every group) it is 317px.
3. **Expand icon: `ChevronsDown` / `ChevronsUp`, as criterion 10.** A single
   `ChevronDown` at the end of a bar is the combobox caret the customer ruled
   out. The double chevron is kept; the demo checks it (§9).
4. **Missing font with Mixed (criterion 3).** If any subject face is missing,
   the icon is `TriangleAlert` and the name is "Font: Mixed, missing"; the
   tooltip's third line is "1 of 3 fonts missing on this computer."
5. **Criterion 12 in the Select bar.** Row 2 is the bottom line there, so Tab
   follows what is seen: expand, Object to path, Linked clone, row 2 (§5). The
   "Tab from the expand button goes to row 2" sentence applies to the Text bar.

## 1. Font jump button (criteria 1 to 5)

- 28 x 28 icon button, the Glyphs button's look: no outline, `rounded-md`,
  hover `--accent-hover`, focus ring 2px `--accent` with 1px offset on
  `--toolbar-bg`. Lucide `Type` 16px, 1.5px stroke, `--toolbar-icon`.
- Missing: `TriangleAlert` 16px in `--field-invalid` (4.8:1 on
  `--toolbar-bg`), same box. The icon, the accessible name and the tooltip
  each carry it, so colour is never the only cue.
- No pressed state: the button jumps, it toggles nothing. Glyphs keeps its
  `aria-pressed` because it opens and closes its pane.
- Tooltip below the bar (`side="bottom"`, 400 ms like every bar tooltip):
  line 1 "Font (Shift+Ctrl+T)" (browser "Font"), line 2 the face name or
  "Mixed", line 3 only when missing. No other lines.

## 2. Row 2 look (criteria 7, 8, 16)

- **One surface.** Row 2 is inside the same pill as row 1: same
  `--toolbar-bg`, same `--panel-elevation-shadow`, same 8px side padding,
  lines of 36px (28px controls, 4px above and below), the same dividers
  between groups.
- **Separator.** A 1px line in the divider colour (`--toolbar-icon` at 25 %)
  across the pill's inner width (8px in from each side) at the top edge of row
  2. It takes no height: it is drawn inside the 4px top padding of row 2's
  first line, so the line counts stay multiples of 36px.
- **Left edge.** Row 2's first control starts at the pill's 8px padding, the x
  of the Font button. Every wrapped line starts there too, in row 1 and row 2.
  Today a wrapped line starts 13px further in, because the divider at a row
  start is `invisible` and keeps its 1px and its 12px gap. Fix (shared by all
  bars that use `Divider`): the flex container's gap becomes 25px and the
  divider is drawn absolutely 13px left of its group, hidden at a row start.
  Widths do not change when a group starts a row, so the row-start
  measurement cannot oscillate.
- **Width.** Row 2 never widens the pill (`w-0 min-w-full` on its container),
  so the centred Text bar does not re-centre and no row 1 line re-wraps when
  row 2 opens (criterion 16). Row 2 wraps by whole groups inside row 1's
  width; a group wider than that wraps its controls inside itself (rule
  "Select bar layout").
- **Select bar.** Row 2 is the bottom line of the Select bar pill, under
  Object to path and Linked clone, with the same separator and left edge. No
  heading: the controls' names say what they are.
- **Notices** of row 1 controls (Size clamp, Style) get `z-10` so they paint
  over row 2 (criterion 17).

## 3. Expand button (criteria 10 to 15)

- 28 x 28, the bar's icon-button look, `ChevronsDown` (closed) or
  `ChevronsUp` (open), 16px, 1.5px stroke, `--toolbar-icon`. No pressed fill:
  the icon is the state.
- A 25px divider before it, like any group. It is always the last control of
  row 1, so it sits at the right end of row 1's last line.
- **Changed dot (criterion 13).** 6px circle in `--accent`, centre 5px in from
  the button's top and right edges, with a 1px `--toolbar-bg` ring so it stays
  apart from the chevron in both themes. Accent on `--toolbar-bg` is 3.3:1
  (non-text minimum 3:1). It appears and disappears at once: no pulse, no
  fade. It counts values only (writing mode, glyph orientation while vertical,
  Taper, later the On path options). Glyphs being open does not count. With no
  text (Text tool before the first text) it reflects the settings the next
  text gets.
- **Tooltip:** "More text options" or "Fewer text options"; while the dot
  shows, a second line "Changed: Writing mode, Taper", the same text as the
  accessible description (`aria-describedby`). It names a state, so it is a
  label, not a hint.

## 4. Motion

None. Row 2 appears and goes in the same frame, the chevron swaps in the same
frame, the dot has no transition. Nothing to change under
`prefers-reduced-motion`. The 120 ms cap of the app is not used here, because
a growing row would move what overlaps the canvas while the pointer is on it.

## 5. Keyboard and focus order

- **Text bar, row 2 closed:** Font, Size, Alignment (one stop, arrows inside),
  Style weight (one stop), Italic, On path, expand, then out of the bar.
- **Row 2 open:** ... expand, Writing mode (one stop), Glyph orientation (one
  stop; greyed buttons stay reachable as today), Taper, Glyphs, On path
  options, then out. Shift+Tab runs the same list backwards.
- **Select bar:** its order with the row 1 text controls in place, then Object
  to path, Linked clone, then row 2 last, matching what is seen. Tab from the
  expand button goes to Object to path, not to row 2; criterion 12 ("Tab from
  the expand button goes to the first control of row 2") holds in the Text bar
  only. In the Select bar `aria-controls` tells assistive technology where row
  2 is.
- Enter or Space toggles and keeps focus on the button (criterion 11). A mouse
  press returns focus to the canvas or the edit sink (criterion 15). Escape on
  any row 2 control returns focus as the Size field does (`0078` §6).
- No shortcut (spec, out of scope). Shift+Ctrl+T stays on the Font button.

## 6. Fit tables

Inner width = overlay row less 16px. 800 x 600 with the panel open and the
second rail column: 324px row, **308px** inner. 1280 x 800, panel open: about
804px row (`0079` UX notes §10.1), **about 788px** inner. Widths: Font 28,
Size 120, Font and Size group 152, Alignment 124, Style 196, On path 28,
expand 28, Writing mode and Glyph orientation group 209 (92 + 25 + 92), Taper
99 (spec stand-in 120), Glyphs 28. On path options of `0080` estimated:
Side 60, Line 60, Position 100, Release 28 with 12px gaps = 284.

### Text bar, 800 x 600 (inner 308px)

| Line | Controls | Width |
|---|---|---|
| 1 | Font, Size, Alignment | 301 |
| 2 | Style, On path, expand | 302 (today without On path: 249) |
| 3 (open) | Writing mode, Glyph orientation | 209 |
| 4 (open) | Taper, Glyphs | 152 (Taper 120: 173) |
| 5 (open, text on path) | On path options | 284 |

Closed 2 lines (72px), open 4 lines (144px), open with a text on path 5 lines
(180px): the last is for `0080`'s UX notes to accept or reduce.

### Text bar, 1280 x 800 (inner about 788px)

| Line | Controls | Width |
|---|---|---|
| 1 | Font, Size, Alignment, Style, On path, expand | 628 (today 575) |
| 2 (open) | Writing mode, Glyph orientation, Taper, Glyphs | 386 (Taper 120: 407) |
| 3 (open, text on path) | On path options | 284 |

Closed 1 line (36px), open 2 lines (72px). With a text on path, row 2 needs
695px but may not be wider than row 1 (628px, §2), so it is 3 lines (108px).

### Select bar, one text selected

| Window | Closed | Open |
|---|---|---|
| 800 x 600 | 5 lines, 180px: switches; Flip and Guide; Font, Size, Alignment (301); Style, On path, expand (302); Object to path and Linked clone | 7 lines, 252px (+ row 2 lines 3 and 4 of the Text bar table) |
| 1280 x 800 | 2 lines, 72px (the text controls shrink by 140 + 234 + 53 − 53 − 53 = 321px against today) | 3 lines, 108px |

These are computed. Criterion 21: the build PR measures them in the browser
build and writes the measured counts into the design-system rows.

## 7. Behaviour with large documents

The dot is computed from the subject values the bar already holds for Mixed;
no extra pass over the document. Opening row 2 renders bar controls only and
does not redraw the canvas. The preference is read once at startup.

## 8. Design-system changes for the build PR

- New rule under "Interaction conventions": **Bar overflow: expandable second
  row, never a dropdown icon**, with the text of criterion 22 and §2 to §5
  here (one surface, separator, left edge, `w-0 min-w-full`, no motion, the
  dot).
- Rows "Font button", "Text bar", "Select bar Text group", "Writing mode and
  Glyph orientation groups" (now row 2), and the Glyphs paragraph of "Text bar"
  updated; "Select bar layout": the divider fix of §2.
- "Keyboard shortcuts established so far": unchanged.

## 9. Questions for the demo

- Does the double chevron read as "more row", not as a dropdown? If not: the
  fallback is Lucide `PanelBottomOpen` / `PanelBottomClose`.
- Glyphs on row 2 (§0.1): accepted, or back to row 1 with 3 lines at 800 x 600?
