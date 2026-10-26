# Text bar second row and Font jump button

Status: Done (PR #188, 2026-10-26; was Ready 2026-10-24, adrs.md and UX notes done)
Priority: Should
Origin: Customer (direction of 2026-10-24); the split of the controls, the icon, the order and the "changed" dot are marked **(Proposal)**

## User value

As a maker who letters signs and names, I want the text controls I use on
every text (font, size, weight, alignment) on one short row and the ones I
rarely use (vertical text, glyph orientation, taper) on a second row that I
open when I need it, so that the bar no longer covers a third of my small
canvas and I still reach every text control without a menu.

Customer direction, 2026-10-24 (translated): "Definitely no collapsible or
dropdown icon. Rather an expandable second row for the functions you don't use
so often. We could also save space: we show the font name quite large; there
we'd only need a small button to jump to the right panel in the Properties."

## Background

Today (branch `story/text-vertical`, `0079` slice 3) the Text bar holds, in
this order: the Font button (168px, shows the face name), Size (120px), Style
(196px), Alignment (124px), Writing mode and Glyph orientation (197px), Glyphs
(28px). Measured: two rows at 1280 x 800, four rows (144px) at 800 x 600. Still
to come: the On path button (`0080` criterion 1). The Taper (`0079` slice 2)
is a field of the text layout block in the Fonts tab (customer decision
2026-10-25), not a bar control. Per-character kerning
(`0079` slice 4) adds no bar control: its fields sit in the text layout block
of the Fonts tab (`0079` criteria 6 and 26) and its keys are Alt+arrows.

How others do it:

| Tool | Where the rare text controls go |
|---|---|
| Inkscape 1.x | Text tool controls bar; on a narrow window the controls that do not fit move into an overflow menu at the right end (a `>>` button with a popup list). The font family is a wide combo box |
| LightBurn | Text options in a fixed toolbar row that is cut off on narrow windows; the font is a wide combo box |
| Affinity Designer 2 | Context toolbar with the font family as a wide field; Character and Paragraph panels for the rest |
| Illustrator | Properties panel with "More options" (`...`) that opens a popup; the Control bar hides what does not fit |

What we do better: nothing is behind a popup or cut off; the rare controls are
a real row of the same bar, one press away, keyboard-reachable in order; the
font name, which every other tool spends 150 to 250px on, lives where the font
list is (Fonts tab), and the bar keeps one 28px button that jumps there.

## Acceptance criteria

Terms. **Text bar**: the Text tool's bar in the overlay row (`0074`). **Select
bar Text controls**: the groups the Select bar shows while the selection holds
a text (`0074` criterion 11, `0078`, `0079`, `0082`). **Row 1** and **row 2**:
the two parts of these controls defined below; each part may wrap by whole
groups as today. **Subject texts**: the texts the controls act on today.

### A. Font jump button

1. Given the Text bar or the Select bar Text controls, then the Font button is
   a 28 x 28 icon button (the Glyphs button's look, Lucide `Type` 16px at
   1.5px stroke) and shows no font name in the bar. **(Proposal: the icon)**
2. Given one subject font, then the button's accessible name is "Font: " plus
   the face name ("Font: Source Sans 3 Regular"); with differing subject fonts
   "Font: Mixed". Its tooltip has the lines "Font (Shift+Ctrl+T)" (browser:
   "Font"), the face name or "Mixed", and, for a missing font, "Missing on this
   computer. Shown in <substitute>." as today.
3. Given a subject font that is missing on this computer, then the icon is
   Lucide `TriangleAlert` in the invalid-field colour instead of `Type`, and the
   accessible name ends with ", missing" ("Font: Lobster Regular, missing").
4. Given any state, when the maker presses the button (mouse, Enter or Space),
   then the Properties panel is shown (expanded if it was collapsed), the Fonts
   tab is active, the row of the current family is scrolled into view and its
   current face row has focus; with "Mixed" or no text, the behaviour of the
   Font button of `0074` for that case is kept. Shift+Ctrl+T (desktop) does the
   same. A second press while the Fonts tab is already shown scrolls and focuses
   again; it never closes the panel or the tab.
5. Given the Fonts tab after the press, then the current face name is readable
   there in full (the tab's subject line, `0079` criterion 6), so no name is
   lost by criterion 1.

### B. Which control is on which row

6. Given the Text bar, then row 1 holds, in this order: Font button, Size field
   (one group, 4px apart), Alignment group, Style group, On path button (once
   `0080` is built), then the expand button of part C. **(Proposal: the split
   and the order; Alignment moves before Style so that Font, Size and Alignment
   fit one 324px line at 800 x 600; Glyphs moves to row 2 and Font and Size are
   4px apart, UX notes §0.1 and §0.2, lead decision 2026-10-24)**
7. Given the Text bar, then row 2 holds, in this order: Writing mode group,
   Glyph orientation group (`0079` slice 3), Glyphs button (while shown, as
   today). When `0080` is built, its options of a text on a path (Side, Line,
   Release from path) go to row 2 after Glyphs unless its UX notes give a
   reason otherwise. **(Proposal)** *(Customer decision 2026-10-25: the Taper is a field of the text layout block in the Fonts tab, `0079` criterion 16, not a row 2 control.)*
8. Given the Select bar with a text in the selection, then its Text controls
   follow the same split: the row 1 controls stay where the Text group is today
   (after Guide, before Object to path) in the order of criterion 6, and the
   row 2 controls are a row of their own at the bottom of the Select bar, under
   every other row (including Object to path and Linked clone). Controls of
   other kinds (rectangle, ellipse, polygon) do not move.
9. Given any control, then it behaves exactly as before this story (values,
   Mixed, greyed states, keys inside the group, tooltips, notices, undo steps);
   only its place changes.

### C. The expand button

10. Given row 1, then its last control is an icon button 28 x 28 with Lucide
    `ChevronsDown` while row 2 is closed and `ChevronsUp` while it is open, in
    the toolbar icon look; no single chevron, no ellipsis, no caret. It opens no
    popup, menu or popover: pressing it shows or hides row 2 in place.
    **(Proposal: the double chevron, to read as "more row" and not as a
    dropdown)**
11. Given the expand button, then it has `aria-expanded` (`false`/`true`),
    `aria-controls` pointing at row 2's container, the accessible name and
    tooltip "More text options" while closed and "Fewer text options" while
    open; Enter and Space toggle it; focus stays on it after the toggle; it has
    no keyboard shortcut.
12. Given row 2 closed, then its controls are not in the DOM and not in the Tab
    order. Given row 2 open in the Text bar, then Tab from the expand button goes
    to the first control of row 2, and Shift+Tab from the first control of row 2
    goes back to the expand button; in the Select bar Tab follows what is seen
    (expand, Object to path, Linked clone, row 2; UX notes §0.5). Row 2 has `role="toolbar"` (Text bar) or `role="group"`
    (Select bar) with the accessible name "More text options".
13. Given a subject text whose row 2 values differ from the defaults (writing
    mode not Horizontal, or a subject text on a path), while row 2 is closed, then the
    expand button carries a 6px dot in the accent colour at its top right corner
    and its accessible description is "Changed: " plus the changed control names
    ("Changed: Writing mode, On path"); with row 2 open, or all values default, no
    dot. Mixed counts as changed. **(Proposal)**
14. Given the maker opens or closes row 2 in the Text bar, then the Select bar
    Text controls show the same state, and vice versa (one preference). The
    state is per user, outside the project file (as the panel's collapsed state
    of `0051`): it survives switching tools, selecting another text, closing and
    reopening the project and restarting the app; the browser build keeps it in
    the browser's local storage. A new user starts with row 2 closed. Opening or
    closing row 2 is not an undo step and does not mark the project unsaved.
15. Given a mouse press on the expand button, then focus returns where the
    other bar buttons return it (canvas or the edit sink, `0078`); in edit mode
    the text stays in edit mode and the caret where it was.

### D. Layout and fit

16. Given row 2 opens or closes, then no row 1 control moves (its x and y on
    screen are unchanged to the pixel), row 2 appears below the last line of row
    1 (Text bar) or of the Select bar (criterion 8), with no animation, and it
    overlays the canvas as wrapped bar rows do today: the canvas view (pan, zoom,
    ruler origin) does not change and nothing else in the window moves.
17. Given a notice that shows under a row 1 control (Size clamp notice, Style
    notice), then it is drawn above row 2 and readable in full.
18. Given the window at 800 x 600, the Properties panel open, the second rail
    column shown (overlay row 324px), a text in edit mode, all controls of
    criteria 6 and 7 present (with On path, or a stand-in of its width, 28px;
    the build measured a 120px Taper stand-in too, which is no longer on the
    bar since 2026-10-25), then the Text bar is 2 lines (72px) with
    row 2 closed and at most 4 lines (144px) with row 2 open; nothing scrolls
    sideways and nothing is clipped.
19. Given the window at 1280 x 800, panel open, the same controls, then the
    Text bar is 1 line (36px) with row 2 closed and 2 lines (72px) with row 2
    open.
20. Given the Select bar with exactly one text selected at 800 x 600 (as
    criterion 18), then it has at most 5 lines (180px) with row 2 closed
    (today: 6 lines, 216px, without Taper and On path); at 1280 x 800 at most 2
    lines (72px) closed and 3 lines (108px) open.
21. Given the build PR, then the line counts of criteria 18 to 20 are measured
    and written into the design-system rows "Text bar" and "Select bar Text
    group".

### E. Design system rule

22. Given `docs/design-system.md` after this story, then it has a rule **"Bar
    overflow: expandable second row, never a dropdown icon"**: when a bar's
    controls do not fit, the rarely used ones form row 2 of that bar, shown and
    hidden by a `ChevronsDown`/`ChevronsUp` button at the end of row 1 with the
    behaviour of criteria 10 to 16; no overflow menu, no `...` or `>>` button,
    no popover for bar controls; frequent controls stay on row 1; row 2 state is
    one per-user preference per bar family (here: text). The rows "Font button",
    "Text bar" and "Select bar Text group" are updated to this story.
23. Given the design-system row "Font button" (`0074`), then it describes the
    icon button of criteria 1 to 4 and no longer the 168px text button.

## Changes to accepted specs

- `0074` criterion 11 and design-system row "Font button": the Font button
  loses its visible name (criteria 1 to 5); the name stays in tooltip,
  accessible name and Fonts tab.
- `0078` and `0079` slice 1: Alignment comes before Style (criterion 6,
  Proposal).
- `0079` slice 3 (criterion 20): Writing mode and Glyph orientation move from
  "after the Alignment group" to row 2.
- `0079` slice 2 (criterion 16): Taper was placed in row 2; on 2026-10-25 the
  customer moved it into the text layout block of the Fonts tab, so row 2 does
  not hold it.
- `0080` criterion 1: the On path button is on row 1; its options go to row 2
  (criterion 7).

## Out of scope

- Row 2 for any other bar (Select bar kind groups of rectangle, ellipse,
  polygon; Node bar; Shape bars). The rule of criterion 22 applies to them when
  a story needs it.
- Letting the maker choose which control is on which row, or reordering by
  drag.
- A keyboard shortcut for the expand button.
- A font family combo box or search in the bar; the font is picked in the
  Fonts tab only.
- Changes to the Fonts tab, the text layout block or the kerning fields.
- Auto-opening row 2 when a vertical text is selected (criterion 13
  shows a dot instead).
- Any change to the document or the project format (no `format_version` bump).

## UX notes

See [ux-notes.md](ux-notes.md). Its §0 proposes five changes to the criteria
(Glyphs on row 2, Font and Size 4px apart, the double chevron kept, missing
font with Mixed, Tab order in the Select bar) and §9 holds two demo questions.

## Links

Requirements: R-EDIT-048, R-FONT-001, R-FONT-011  PR: #188
Depends on: `0079` slice 3 (`story/text-vertical`) merged. Related:
`0074`, `0078`, `0079`, `0080`, `0082`, `0051` (per-user panel state).
