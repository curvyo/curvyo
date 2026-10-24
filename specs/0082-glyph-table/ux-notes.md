# 0082 Glyph table: UX notes

ux-engineer, 2026-10-24. Written for Frage 39 A and Frage 40 A. Checked against `docs/design-system.md` (rows "Font preview (`0077`)", "Text bar", "Select bar Text group", "Style group", "Font button", "Tag strip", "Presets" (preset strip look), "List row", "Slider", "Action notice", "Hidden, not disabled", "Row stability", "Text edit mode") and `specs/0077-font-preview/ux-notes.md`, cited below as "0077 §n".

Glyphs is a second mode of the `0077` pane, not a new surface. It adds **one radio strip in the pane header, one button in the Text bar and the Select bar, and a footer bar in the pane**. The pane itself (place, split, full view, divider, open and close, grounds) is 0077 §1 unchanged. No popup, no tooltip on cells, no new global key, no hint label.

## 0. Proposed changes to the specification

| # | Criterion | Change | Why |
|---|---|---|---|
| P1 | 2 | The Glyphs button is a **group of its own after the Style group**, in the Text bar and in the Select bar Text group (exactly one text selected), not inside the Font and Size group. | Font, Glyphs and Size are 348px, which does not fit the 324px overlay row at 800 x 600; after the Style group it fits there (249px) and costs no extra row in either bar. The Select bar shows Font and Size for a text too, so the button belongs beside them there as well. |
| P2 | 15 | Cells are squares of side S = max(40px, 2 x size) with a **4px gap**, columns of exactly S, laid from the left (no stretching); the space left over (less than one cell) stays at the right. | Stretched cells become rectangles and move the glyph off its column when the pane is resized. 12px gaps (the tile grid) make a table of 40px cells look sparse. |
| P3 | 18 | The Recent row shows **at most two rows** of cells (2 x columns, at most 24); the rest stays in the list. It is not shown while the search holds text. Its order does not change while the pointer is over it; it re-sorts when the pointer leaves it. When it appears or grows, the scroll position is moved by the same height, so the cell under the pointer or the focus does not move (row "Row stability"). | 24 cells at 3 columns would be 8 rows of Recent before the first block. A click must never move the next cell under the pointer. |
| P4 | 19 | The detail bar is **always 97px high** (1px line + 96): the glyph box is 80 x 80px with the glyph at cap height 40px whatever the Size, so the bar never changes height. It also shows the block name and an **Insert** button (§5). | A bar that grows with the Size slider moves the grid's bottom edge; 2 x size would make the detail smaller than the cell from size 48 on. |
| P5 | 20 | Ctrl+Up and Ctrl+Down move the focus to the first cell of the previous or next block (the block jump; no list control, as Out of scope says). Down or Enter in the search field moves the focus to the first cell. Backspace in the grid goes to the search field like a printable character. | A block jump without chrome; search, then arrow, then Enter is one keyboard flow. |
| P6 | 6 | Block header rows are **sticky**: the header of the block at the top of the grid stays at the top while its cells scroll under it. | In a 3,000-character face the maker otherwise loses where they are. |
| P7 | 4, new | Subject line with a search: "Great Vibes Regular · 3 of 412 characters" (the Fonts mode's "Cuttable · 3 of 42" pattern). Missing face: "Lobster Regular · missing". A face with no listed character: grid text "Mylines Regular has no characters to insert." | Same words as the Fonts mode; the third case is otherwise a blank grid. |

## 1. Entry points

| Item | Decision |
|---|---|
| Mode switch | First control of the header (§2). The preset strip look (row "Presets": 28px, 1px `--toolbar-icon` at 60 % border, `rounded-[5px]`, cells 1px apart, 14px text, 6px side padding, minimum 32px): cells **Fonts** and **Glyphs**, about 104px. `role="radiogroup"` "Preview mode", cells `role="radio"`, one Tab stop on the pressed cell; Left and Right move and choose (no wrap), Home and End. Pressed, hover and focus as the strip. A mouse press returns the focus to the canvas or the edit sink; a key press keeps it on the strip. `0083` adds **Info** as a third cell (about 143px then). |
| Glyphs button | Text bar and Select bar (P1). 28 x 28 icon button, Lucide `Omega` 16px, 1.5px stroke, the look of the Style group's Italic toggle: idle `--toolbar-icon`, hover `--editor-accent-hover`, pressed `--toolbar-icon-active-bg` / `-fg` while the pane is open in Glyphs mode, `aria-pressed`. Name and tooltip "Glyphs" (Radix, 400 ms, bottom). In the tree only while a text is in edit mode or exactly one text is selected (row "Hidden, not disabled"). Press: opens the pane in Glyphs (or switches it to Glyphs); pressed again, closes the pane. Mouse: focus back to the edit sink or canvas. Enter or Space: focus to the search field. |
| Bar layout | Text bar: [Font, Size] divider [Style] divider [Glyphs]. 1280 x 800: one row, 590px. 800 x 600: two rows, 72px as today (Font and Size; Style and Glyphs). Select bar: Glyphs 12px after the Style group on the Style row; rows unchanged. |
| Reopen | Last mode, Glyphs button always Glyphs (criterion 3). The split width is computed by 0077 §1 in both modes (one tile column at the Size), so switching modes never moves the divider. |

## 2. Header

0077 §2 with two changes: the mode switch is first in row 1 in both layouts, and in Glyphs mode the sample field is the search field. Size slider, readout, Full view, Close and the subject line stay where they are; the Size value is one shared value.

| Width of the pane | Layout |
|---|---|
| **Under 560px** | Two rows, 81px with the line. **Row 1:** mode switch 104, 8, field (flex; 147px at the 339px split, 90px at the 282px minimum), 4, Full view 28, 4, Close 28. **Row 2:** as 0077 §2: "Size", slider 96, readout 44, 12, subject line (flex, ellipsis). |
| **560px and wider** | One row, 45px: mode switch, 12, field (200 to 480px), 16, "Size", slider 120, readout 44, 16, subject line (flex), 8, Full view, 4, Close. |
| Search field | The Fonts tab search field look **with** its `Search` icon 16px (it is a search, the sample field is not): white, `rounded-[5px]`, 1px `ring-border`, 14px, `maxLength=64`, `aria-label="Search characters"`, no placeholder, "Clear search" `X` 16px inside the right edge while it holds text. Its own stored text. Escape closes the pane and keeps the text (0077 P5). Down or Enter: focus to the first cell (P5). |
| Subject line | 12px `--panel-muted-fg`: "Great Vibes Regular · 412 characters", "Great Vibes Regular · 3 of 412 characters", "Lobster Regular · missing" (P7). A state label. |
| Tab order | Divider (split only), mode switch, field, Size slider, Full view (when present), Close, grid, Insert. |

## 3. Grid and cells

| Item | Decision |
|---|---|
| Ground | `--canvas-bg`, 12px padding, as the Fonts grid. |
| Cell | Square of side S (P2), `--tile-bg`, 1px `--tile-edge`, `rounded-[5px]` (the tile's 8px is too round on a 40px square), no shadow, no text. The glyph from the subject face's outlines as one `<path>` in `--toolbar-icon` (`currentColor`), cap height = size, baseline 0.68 S from the top (0.18 S above the cap line, 0.32 S for descenders), centred on its advance, clipped at the cell's edge. Single-line glyphs: the 0077 tile stroke (1 CSS px, non-scaling, round, never filled). |
| Empty outline | A 1px dotted `--panel-muted-fg` box as wide as the advance (at least 4px, at most S - 8) from the baseline to the cap line, and under it, centred, a 10px tabular label in the UI font, `--panel-muted-fg`: the Unicode abbreviation alias of the character (`NameAliases.txt`, type "abbreviation": SP, NBSP, ZWSP, ZWNJ, ZWJ, WJ, SHY, NNBSP, MMSP, IDSP, LRM, RLM, CGJ, ZWNBSP, ...), else the hex code point "2002". One rule, no hand-kept list. |
| Hover | `--editor-accent-hover` ground, cursor `default`, no lift. The detail bar shows the cell (§5). No tooltip: the bar is the one place for names. |
| Focus | 2px `--editor-accent` ring inside the cell. The focused cell is also the **current cell** (roving `tabindex="0"`); a mouse click makes the clicked cell current. No other persistent mark (nothing is "picked" in this mode). |
| Loading | A cell not drawn yet: a `--tile-skeleton` bar, cap height high and 0.4 S wide, on the baseline, 3px radius (the 0077 loading look). No shimmer. Swapped in place. |
| Block header | A 28px row across the grid: "Latin-1 Supplement · 96", 12px, weight 600, `--panel-muted-fg`, from the grid's left padding, text bottom 6px above the first cell row. Sticky (P6) on an opaque `--canvas-bg` ground, no line, no shadow. `role="group"`, `aria-labelledby` the header. 12px between the last cell row of a block and the next header. |
| Recent | The first group, header "Recent · 12", same look and stickiness; cells as any cell, drawn in the subject face; at most two rows; absent while empty or while the search holds text (P3). |
| Semantics | `role="listbox"` "Characters of Great Vibes Regular", groups as above, cells `role="option"` with the name of criterion 20 ("Latin small letter a with diaeresis, U+00E4"); `aria-selected` follows the focus; `aria-setsize` and `aria-posinset` for the virtualised cells. |

## 4. Click, Enter and keyboard

| Situation | Click (or Enter or Space on the focused cell) |
|---|---|
| A text in edit mode | Inserts at the caret, replacing the text selection (criterion 12). |
| One text selected, not editing | That text enters edit mode, caret at the end of its last line, then inserts (criterion 13). |
| Anything else | A new text with the character in the next-text face at the visible centre, in edit mode, caret after it; one step "Add text" (criterion 14). |
| Missing face | Nothing to click (empty grid, §6). |

After a mouse click the focus goes to the edit sink, so typing goes on; after Enter or Space it stays on the cell. The insert is announced in a polite live region ("Inserted ä"); no visible notice: the caret moves on the canvas, and the Recent row takes the character. A double-click is two inserts, as typing twice.

**Grid keys.** Left and Right move by one cell in reading order, across row ends and block headers. Up and Down move by a visual row in the same column, across headers; to a shorter row, its last cell. Home and End: first and last cell of the row. Ctrl+Home and Ctrl+End: first and last cell. Page Up and Page Down: by the grid's visible height. Ctrl+Up and Ctrl+Down: first cell of the previous or next block (P5). Enter or Space: insert. A printable character or Backspace: to the search field, which takes it. Escape: closes the pane (0077). Tab: on to Insert. Moving the focus scrolls the cell into view below the sticky header.

## 5. Detail bar

| Item | Decision |
|---|---|
| Place | The pane's last row, full width, always present in Glyphs mode, 97px: 1px `--toolbar-icon` at 25 % top line (the header's line, mirrored), `--panel-bg`, 8px padding. Not in Fonts mode. |
| Shows | The hovered cell; with no hover, the current cell; else nothing (the bar stays, empty). Updates instantly, no fade. |
| Layout | Glyph box 80 x 80 (a cell look: `--tile-bg`, 1px `--tile-edge`, 5px radius, cap height 40px, P4), 12, text column (flex), 12, Insert. Text column, top aligned: Unicode name 14px `--toolbar-icon`, up to two lines, then end ellipsis ("LATIN SMALL LETTER A WITH DIAERESIS"); 4px; one 12px tabular `--panel-muted-fg` line "U+00E4 · adieresis · Latin-1 Supplement" (glyph name left out when the face has none), ellipsis. |
| Copy | Name and meta line are selectable text (`user-select: text`). The glyph box carries the character as transparent selectable text over the drawing, so selecting across the bar copies "ä LATIN SMALL LETTER A WITH DIAERESIS U+00E4 ...". |
| Names loading | Until the character names are loaded (criterion 17) the name line is a `--tile-skeleton` bar 60 % wide, 14px high; the meta line shows at once. |
| Insert | 28px outline text button, the "Fit to content" look, label "Insert", 12px side padding, vertically centred, at the right edge. Inserts the character the bar shows, by §4's table. In the tree only while the bar shows a character; the bar's height and the text column's width do not change (the slot is reserved). Mouse: focus to the edit sink; key: stays on Insert, so Enter repeats the insert. |

## 6. Empty states

All in the 0077 empty-grid look: 14px `--panel-muted-fg`, centred, 24px from the top of the grid, one line, no second line. The detail bar stays, empty.

| Case | Text |
|---|---|
| Missing face (criterion 11) | "Lobster is missing. Its characters cannot be shown." |
| No listed character (P7) | "Mylines Regular has no characters to insert." |
| Search without hits | "No character in Great Vibes matches "xyz"." |
| Names still loading, only a name search would match | "Loading character names…" |
| Face being read (first open) | The sticky header row "Basic Latin" is not shown yet; the grid shows skeleton cells for one screen, no text. |

A face with few characters (a dingbat font with 30, a single-line font with 95) is not a special case: its blocks and cells, then the `--canvas-bg` ground. The cells keep their size; nothing stretches to fill the pane.

## 7. Fit

Canvas region and heights as 0077 §6. Header 81px (two rows) or 45px (one row), detail bar 97px. Grid width = pane less 24px padding, 17px scrollbar gutter and the 1px divider line in split view. Columns = floor((width + 4) / (S + 4)).

| Window | View | Pane | Grid area | Size 48 (S = 96) | Size 24 (S = 48) |
|---|---|---|---|---|---|
| 800 x 600 | Full (forced) | 488 x 576 | 447 x 398 | 4 columns, about 14 cells visible | 8 columns, about 56 |
| 1280 x 800 | Split | 339 x 776 | 297 x 598 | 3 columns, about 17 | 5 columns, about 55 |
| 1280 x 800 | Full | 968 x 776 | 927 x 634 | 9 columns, about 54 | 17 columns, about 200 |

Detail bar text column: 154px at the 339px split (the name wraps to two lines for long names), 98px at the 282px minimum, 783px in full view at 1280.

## 8. Design-system rows to add (by the implementer with the slice)

In "Font preview (`0077`)": a row "Mode switch" (§1), a row "Glyphs mode" (§2 to §7, one row each as built), amend "Header" (mode switch first, search field in Glyphs mode) and "Keys" (§4). In "Text bar" and "Select bar Text group": the Glyphs group after the Style group, with the measured rows. Keyboard shortcuts table: "Glyph table" row for §4's keys.
