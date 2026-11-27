# 0146 Material test picks: UX notes

ux-engineer, 2026-11-26. Written from `specification.md` (criteria 1 to 25, Fragen 312 to 316 at their default A), `docs/design-system.md` (rows "Link corners toggle", "Mode switch" (the preset strip look), "Casing over artwork", "Command glyphs"; "Interaction conventions"), the built notes `specs/0112-test-grid-and-materials/ux-notes.md` (sections 2, 3.3, 7.4, 8.2, 8.3; this story revises them where section 12 says), `specs/0028-material-test-library/ux-notes.md` (8.2 Material pick, 8.4 states, 7.3 the old Worked chip), `specs/0111-laser-operation-tools/ux-notes.md` 6.2, `specs/0101-laser-operations/ux-notes.md` 13 (the hatch) and the built code (`JudgeTable.tsx`, `TestDetail.tsx`, `MaterialRow.tsx`, `operation_hatch.rs`, `viewport.rs`). Customer rules applied: no hint texts, no popups in the panel, no success notice, kind is shape, colour only helps. Items marked (Proposal) are vetoable at the demo; section 11 lists them as P1 to P12.

Terminology as in the spec: **pick**, **pick kind**, **pick set**. "Ink" is `--toolbar-icon`. "Pick control" is the shared component of the pick bar (test detail) and the pick pick (operation).

## 0. What is reused

| Part | Reused from | Change in this slice |
|---|---|---|
| Judge table | `JudgeTable.tsx` (`role="grid"`, roving focus, 24px rows, cell widths of `0028` 7.1) | four kinds, unmark, frames per kind, quadrant glyphs |
| Test detail | `TestDetail.tsx` (`0112` 8.2) | heading "Picks", the pick bar above the table, one line per pick under it |
| Set block of a laser | `MaterialBody.tsx` (`0112` 7.4 item 3) | the read-only pick lines under each set |
| Operation settings | `OperationSettingsBlock.tsx`, `MaterialPick.tsx` (`0028` 8.2, 8.4; `0111` 6.2) | the pick pick under the Material pick; the state lines name the kind |
| Segment look | preset strip look (28px, 1px ink at 60 %, `rounded-[5px]`), pressed = `--toolbar-icon-active-bg` / `-fg` (as `PanelRadioGroup`) | four symbol-and-word cells in a 2 x 2 grid |
| Canvas chip | the `0028` 7.3 chip (`--toolbar-bg` ground, 1px `--lane-line` border) and "Casing over artwork" | one chip row per marked cell, size rules of 6.3 |
| Engrave look | `operation_hatch.rs` (1.5px lines every 6px, 45 degrees, screen space) | test-grid Engrave cells also draw a 1px `--engrave-hatch` edge (7.2) |

No new component library, no popup, no new canvas layer type beyond the overlay of section 6.

## 1. Principle

The maker reads a burned board and taps the cells he judged, one kind at a time: choose the kind, tap the cell. Each kind has its own symbol everywhere (bar, table, canvas, material, operation), so the symbol is the word in a small space; the colour is a second, weaker cue. A job then says which of the four it follows, with the same four buttons.

## 2. Tokens and glyphs

### 2.1 Pick tokens (new, `docs/design-system.md` "Color tokens")

Light values. Grounds measured: `--panel-bg` `#DCDCE0`; the selected material body `--value-fill` `#C5D7FA` (the test detail and the operation body sit on it, `MaterialRow.tsx`); a hovered cell `--editor-accent-hover` over it, `#A7C2F8`; `--canvas-bg` `#E8E8EB`; the white casing; `--pasteboard-bg` `#B8B8BE`. WCAG 2.x relative luminance.

| Token | Light | panel | value-fill | hover | canvas | white | pasteboard |
|---|---|---|---|---|---|---|---|
| `--pick-best` | `#1B7232` (green) | 4.39 | 4.14 | 3.35 | 4.91 | 6.00 | 3.04 |
| `--pick-second` | `--pick-best`, an alias; its frame is **dashed** 3 on 2 | 4.39 | 4.14 | 3.35 | 4.91 | 6.00 | 3.04 |
| `--pick-fastest` | `#065774` (petrol) | 5.85 | 5.52 | 4.47 | 6.55 | 8.01 | 4.06 |
| `--pick-deepest` | `#B8008C` (magenta) | 4.49 | 4.24 | 3.43 | 5.03 | 6.15 | 3.11 |

- Every frame and coloured glyph reaches 3:1 on every ground it is drawn on (criterion 6). The pick colours are **never used for text**: words and numbers stay ink (7.8:1 on `--value-fill`), so the 4.5:1 text rule is met by ink, not by a pick colour.
- **Why Second best is the Best colour, dashed.** A paler green cannot reach 3:1 on `--panel-bg` (`#6B8E23` measured 2.78:1), and the three 3:1 yellow-greens tried measured 6 to 8 Delta E (CIE76, Machado 2009 simulation) from `--cut` under deuteranopia. Best and Second best are one family ("good"); the "2" glyph and the dashed frame tell them apart.
- **None equals** `--accent` `#2F6FEE`, `--cut` `#C8102E` or `--engrave-hatch` `#C46A1C`. Smallest Delta E to those three / between the picks: normal vision 61 (Fastest to accent, Deepest to cut) / 59 (Best to Fastest); deuteranopia 18 (Best to cut) / 15 (Fastest to Deepest); protanopia 19 (Best to cut and to hatch) / 20; tritanopia 22 (Fastest to accent) / 12 (Best to Fastest). The pairs that come close under colour blindness are carried by the glyph and the dash, as criterion 6 requires; violet was rejected for Fastest because it measured 15 to 19 from `--accent` under deuteranopia and protanopia (it would read as the focus ring).
- **Near hues elsewhere, accepted:** `--guide` `#166534`, `--sketch-solved` `#12733A`, `--marquee-touch` `#1C9347` (greens), `--peer-2` `#0F766E` (teal), `--peer-3` `#BE185D` and `--peer-6` `#A21CAF` (magentas). They live on guide paths, sketches, marquee drags and collaboration marks, never in a test table or around a test cell; the pick glyph is always next to a pick colour.
- **Dark values (provisional).** Only the light theme exists (no ADR). For the value swap, measured on the assumed dark panel ground `#2A2A2E` (the `--field-derived-fg` precedent), a dark `--value-fill` (`--accent` at 28 % over it, `#2B3D64`) and its hover (`#2C4780`): `--pick-best` `#5CC27B` 6.44 / 4.84 / 4.08; `--pick-fastest` `#5BB8DB` 6.35 / 4.77 / 4.02; `--pick-deepest` `#F17ACF` 5.73 / 4.31 / 3.63; `--pick-second` the Best alias. Re-measure when a dark-theme ADR fixes the grounds.
- **On the canvas** the light values are used in both themes: the page ground is the stored document background, not the theme, and every frame has a white casing, so the better of frame-to-ground and frame-to-casing is at least 6.00:1 (the "Document background" rule of the design system).

### 2.2 Glyphs (one per kind, everywhere)

| Kind | Glyph | Notes |
|---|---|---|
| Best | Lucide `Check` | as built for the best cell |
| Second best | custom `SecondBestGlyph` (24-unit viewBox, Lucide stroke rules): `Check` scaled to 75 % at the lower left, a "2" drawn as a path (not font text) in the upper right, 9 units high | the "2" reads from 14px; at 10px and below the dashed frame and the quadrant carry it |
| Fastest | Lucide `ChevronsRight` | not used elsewhere; distinct from the single `ChevronRight` of a test row |
| Deepest | Lucide `ArrowDownToLine` | not used elsewhere; not `ArrowDown`, which is the material row's Down button in the same tab |

Glyphs are 1.5px absolute stroke at 12px and more, 1px below 12px. In buttons they are 16px; in pick lines and canvas chips 12px; in the table 16px (one kind) or 10px / 8px (several, 4.3).

## 3. The pick control (pick bar and pick pick)

One component, `PickControl`, in two places (sections 4 and 5).

### 3.1 Layout

- **2 x 2 grid** of four cells, 112 x 28px each, 4px gaps, 228px wide, 60px high; order Best | Second best, Fastest | Deepest (criterion 1 asks for one row; measured, it does not fit: P1, M1).
- Cell: preset strip look (1px ink at 60 % border, `rounded-[5px]`), glyph 16px, 4px gap, word 12px, left-aligned with 8px padding. Words: "Best", "Second best", "Fastest", "Deepest" (the widest, "Second best", is about 64px at 12px; 8 + 16 + 4 + 64 + 8 = 100 of 112px).
- **Unpressed:** transparent ground, glyph in the kind's pick colour (the legend for the table frames), word ink. Hover `--editor-accent-hover` (pick colours 3.35:1 or more on it, 2.1).
- **Pressed:** `--toolbar-icon-active-bg` fill, glyph and word `--toolbar-icon-active-fg` (white, 4.5:1), word semibold. The pressed state is carried by the fill (3.32:1 to the panel, 3.13:1 to `--value-fill`: a luminance and shape change, not a hue change), the weight and `aria-pressed`; not by the pick colour.
- **Dimmed** (pick pick only, criterion 15): transparent ground, border ink at 25 %, glyph and word `--lane-line` (3.08:1 on the panel, 2.91:1 on `--value-fill`; an inactive control, exempt from the contrast minimum), `aria-disabled="true"`, tooltip and accessible description "No Fastest set for this laser". It stays in the roving focus so a keyboard user reaches its reason; a press, Space or Enter does nothing. Not hidden (the maker sees why).
- **Pressed and missing** (state (c) of `0028` 8.4: the linked kind has no set any more): the pressed look stays (it is the stored link); the state line of 5.2 names the kind. It is never dimmed.
- **Mixed** (pick pick with several operations whose kinds differ): no cell pressed; the group's description "Mixed".

### 3.2 Keyboard and semantics

- `role="group"` with four toggle `button`s (`aria-pressed`), exactly one pressed (none when mixed); one Tab stop on the pressed cell (on Best when mixed).
- **Manual activation:** Left, Right, Up, Down move the focus in the 2 x 2 grid (no wrap), Home and End to the first and last cell; **Space or Enter presses**. Arrows never press, because in the pick pick a press is an undo step that clears the overrides (5.3); the pick bar uses the same rule so the component has one behaviour.
- A mouse press keeps the focus on the pressed cell (the next action is in the table or the fields; the canvas is not the target here).
- In the pick bar a press on the pressed kind does nothing and it stays pressed (one kind is always chosen). In the pick pick it is the step of 5.3.

## 4. Test detail: the pick bar, the table, the lines (criteria 1 to 7)

### 4.1 Order (amends `0112` 8.2 item 5 and 6)

1. Heading **"Picks"** 12px semibold (was "Best cell"; P2).
2. The **pick bar** (3), `aria-label` "Pick kind", Best pressed when the detail opens; the pressed kind is session state, not a step, and resets to Best on the next open.
3. 8px, the **table** (4.2).
4. The **pick lines** (4.5), then Note, Photo, Place as built.

### 4.2 The table

- `role="grid"` named **"Picks of K7Q2M"**, `aria-description` "Marking Fastest" (the pressed kind, criterion 1), updated on every press in the bar.
- Sizes as built: 24px rows; cells 24px wide up to 8 columns, sharing 204px from 9 (17px at 12).
- **Press, Space or Enter** on a cell (criterion 2, 3, 4):
  - the cell does not carry the pressed kind: it gets it, any other cell loses it, the kind's pick set is written; one step **"Mark pick"** for all four kinds (P3; the rename of "Choose best cell" that criterion 2 allows, M3);
  - the cell carries the pressed kind: the kind is removed; one step **"Unmark pick"** (criterion 3; replaces `0112` G14 "a press on the best cell does nothing").
- Polite status (the built `role="status"`): "Fastest: 100 %, 6000 mm/min" on a mark, "Fastest removed" on an unmark.
- Hover `--editor-accent-hover`; focus the built 2px inset `--editor-accent` ring at the cell's edge (the frame of 4.3 sits inside it, so ring and frame never overlap).
- Tab order: Back, head, trash, pick bar, table, Note, Photo (criterion 4: the bar before the table).

### 4.3 Cell look (criterion 6)

| Cell | Look |
|---|---|
| Unmarked | as built: 12px square outline, 1.5px `--lane-line`, centred |
| Frame | replaces the square outline: 2px in the pick colour of the cell's first kind in the order Best, Second best, Fastest, Deepest (dashed 3 on 2 only when that first kind is Second best). It sits inside the built 2px inset focus ring: outer box `min(w − 4, 20)` x 20px centred, inner box `(outer − 4)` x 16px; 24px cells (w = 24): 20 x 20, inner 16 x 16; 17px cells: 13 x 20, inner 9 x 16 |
| One kind, 24px cells | its glyph 16px ink, centred, filling the inner box |
| Several kinds, 24px cells | the glyphs in **fixed quadrants** of the inner 16 x 16 box, 8px each, 1px stroke: Best top left, Second best top right, Fastest bottom left, Deepest bottom right; a missing kind leaves its quadrant empty |
| Narrow cells (9 to 12 columns, 17 to 23px) | the first kind's glyph only, 12px ink, centred, drawn over the frame where it is wider than the inner box (the built Check is drawn over its square the same way). Further kinds are in the tooltip, the name and the pick lines only (M2) |

- The quadrant is a non-colour cue of its own: the same kind sits in the same corner of every cell (P4; criterion 6 says "side by side", M2). The 8px glyphs are the smallest legible size; the review checks them at 1x.
- No word or initial inside a cell: 24px and 17px have no room for one (M2). The word is in the pick bar above (the legend), in the cell's tooltip ("Best, Fastest", 400ms, every marked cell) and accessible name, and in the pick lines under the table.
- Cell name: "60 %, 1950 mm/min" plus ", Best, Fastest" for its kinds; `aria-pressed` is true when the cell carries the pressed kind.

### 4.4 Not Lasered, damaged

- Marking works whether the test is Lasered or not (criterion 5); nothing in the look changes.
- A damaged test (criterion 23) is not opened (the `0028` 10.4 row look); no partial pick is shown.

### 4.5 Pick lines under the table (criterion 7)

- One line per marked kind, in the kind order, 4px apart, each wrapping: kind glyph 12px in its pick colour, 4px, text 13px ink tabular. Best: "Best: Cut set of Diode 20 W: 60 %, 1950 mm/min, 1 ×" (the built set line with the kind word in front); the others: "Second best: 50 %, 1800 mm/min, 1 ×", Engrave and Image also ", 0.10 mm".
- Kinds without a pick are not listed; no pick at all: nothing (the heading and the bar are the cue).
- The numbers are the cell's. When the material's set of that kind came from another test of the same laser (criterion 3), the line ends with " · set from Test 9XK2P" 12px muted, so the maker sees which test the jobs follow.
- After an unmark the line goes. The set follows criterion 3: removed (Second best, Fastest, Deepest) or kept as "Typed" (Best) only when it came from this test.

## 5. The operation's pick pick (criteria 13 to 16)

### 5.1 Place

- In the upper part of the unfolded operation (`0111` 6.2), **directly under the Material pick**, 4px below it, before Power and Passes: the pick control (3), `aria-label` "Use" (the name of the spec; no visible caption, P5).
- Engrave + Outline has one pick pick for both parts (criterion 13); a kind is dimmed when the material has neither that pick's Score nor its Engrave set for the laser.
- Only while the operation points to a material. "Own numbers" has no pick pick. A derived cell operation of a test grid has no Material pick and no pick pick (criterion 16, `0112` G10).
- **Several operations selected:** under the multi-row Material pick (`0028` 8.2), the same control. A kind is dimmed only when **no** selected operation's material has that set for its laser; otherwise the press applies to all in one step and skips the others with the built Action notice: "Cut 2 skipped: Birch 3 mm has no Fastest set." or "2 operations skipped: no Second best set." (45 and 41 characters).

### 5.2 States (criterion 13; `0028` 8.4 with the kind)

| State | State line (12px ink, `Link2Off`) |
|---|---|
| (a) found | none |
| (b) material not found | "Material not found, using stored numbers" (unchanged) |
| (c) found, kind has no set | "Material has no Fastest set for this laser, using stored numbers." (65; Second best 69 characters). For Best the built line keeps its words |

The Differs line, the "Material 60" lines and Reset to material work as built; "Material" there means the linked pick's numbers.

### 5.3 Press

- A press on any kind, **also the pressed one**, is one step **"Use pick"**: it writes the link with that kind and the numbers as last seen and clears every typed number (criterion 14, ADR 0093 §7; the rule of picking the linked material again in `0028` 8.2). No arming: undo is one press away, and the Reset rule already behaves so.
- Focus stays on the pressed cell. No notice (no success notice).
- An operation stored without a kind shows Best pressed (criterion 15).

## 6. Canvas overlay (criterion 8, Frage 316 A)

### 6.1 When

- A test grid that is on a laser page and **selected** shows the picks of its current test; it disappears when the grid is not selected. Both views (Operations and Drawn): it is a record of the test, not an operation look.
- View only: `pointer-events: none`, no hit test, never output, never exported, never in the operations, not in the accessibility tree (the table is the accessible route).
- A Lasered grid that is changed gets a new test id (`0112` criterion 11) with no picks: its overlay is empty.

### 6.2 Frames

- One frame per marked cell around the cell's box (the Cell size square, also around a circle), 2px in the pick colour of its first kind (light values, 2.1), Second best dashed 3 on 2 in screen px, over a 1px `--selection-casing` on both sides.
- Offset: the frame's inner edge sits `clamp(g / 2 − 3, 1, 2)` screen px outside the box, where `g` is the 2 mm gap on screen and 3 is the stroke plus the outer casing. At 100 % (3.78 px per mm, `PX_PER_MM_AT_100`) `g` is 7.6px and the offset 1px: two framed neighbours keep 1.6px between their frames and their casings just overlap. Below 100 % the frames of neighbours move closer and touch from about 79 % (the casing keeps each one visible). The 1px minimum keeps the inner casing off a Cut cell's line. Constant screen width at every zoom.
- Drawn above the grid's look and the selection box, below handles and the Move chip.

### 6.3 Chips

- One chip row per marked cell, centred on the cell: 16px high, `--toolbar-bg` ground (the theme's), 1px `--lane-line` border, 8px radius, 2px padding, 12px ink glyphs 2px apart in the kind order: 16, 30, 44 or 58px wide for 1 to 4 kinds. Constant screen size (it covers the cell on purpose, as the `0028` chip: the mark is the point).
- Size rule, with `p` the cell pitch on screen (Cell size + 2 mm):
  - the full row fits (`width ≤ p`): the full row;
  - else a 16px chip fits (`p ≥ 16`): the first kind's glyph only;
  - else no chip; the frames stay (the table and the pick lines carry the kinds; the overlay is a locator at that zoom, P7).
- Examples: 3 mm cells (pitch 18.9px at 100 %): one glyph from 85 % zoom, four from 307 %. 10 mm cells (45.4px at 100 %): one glyph from 35 %, four from 128 %.

## 7. The round Engrave mark on screen (criteria 18 to 22)

### 7.1 Numbers at 3 mm Cell size

- 100 % zoom is 96 / 25.4 = 3.78 CSS px per mm. A 3 mm circle is 11.3px across at 100 %, 5.7px at 50 %, 22.7px at 200 %; a bed fitted to a 900px high canvas (400 mm bed) is about 56 %, 6.4px.
- The Engrave hatch is screen space: 1.5px lines every 6px at 45 degrees. An 11.3px circle holds 1 or 2 hatch lines, a 6px one 0 or 1. **With the hatch alone a 3 mm circle reads as a slash or as nothing**, below about 24px across (6.4 mm at 100 %, 3 mm at 210 %) it does not read as round. The built square cell had the same weakness; the circle makes it visible, because criterion 22 lets the shape alone say "circle".

### 7.2 The edge (P8)

- A test-grid Engrave **circle** cell draws, in addition to its hatch, a **1px screen-space edge in `--engrave-hatch`** (3.17:1 on `--canvas-bg`) along its outline, at every zoom, in View: Operations. View only: not output, not exported, nothing in the job changes (the hatch clip of `0101` is the output, criterion 19).
- It is not the Engrave + Outline look: that outline is ink, and a test grid never holds an Engrave + Outline cell. Old square Engrave cells keep their built look until the next bar change makes them circles (criterion 21: an old grid opens as it was, also on screen).
- With the edge a 3 mm circle reads as a ring from about 6px across (3 mm at 53 % zoom, close to a fitted bed); below that every 3 mm mark of the grid, Cut squares included, reads as a dot, and the labels carry the grid.
- Crosshatch on: the second hatch direction as built; the edge is the same.
- At large zoom the hatch clip is the 32-point flattening of `operation_hatch::flatten` (8 steps per arc): at 10 mm and 800 % the chord error is 0.7px; the edge is drawn from the four cubics themselves, so the outline stays exact.

### 7.3 Panel and labels

- The cell look in the Operations tab (the kind glyph of the cell rows) does not change; no word, tooltip or setting names the circle (criterion 22). Labels and pitch unchanged (criterion 18).

## 8. Materials tab: pick lines under the sets (criterion 9)

- Under each set of a laser (`0112` 7.4 item 3, after its fields), the picks of that kind in the order Second best, Fastest, Deepest, each a **two-line read-only item**, 4px apart, indented 8px:
  - line 1 (20px): kind glyph 12px in its pick colour, 4px, the kind word 12px ink, at the right the source 11px muted "Test K7Q2M";
  - line 2 (16px): the numbers 12px ink tabular, aligned with the word: "50 %, 1800 mm/min, 1 ×" (Engrave and Image ", 0.10 mm"; at most about 180px of 212px).
- Two lines because one line needs about 290px (glyph, word, numbers, source) of 228px (P9).
- No field, no add button, no trash, no press (criterion 9: change a pick by marking another cell). Not a Tab stop; the set's group description lists them ("Second best 50 %, 1800 mm/min, 1 pass, from Test K7Q2M").
- A kind with picks but no main set (Best never marked or its set deleted): the lines sit under that kind's "+ <Kind> set" button.
- The material row's line 2 ("2 lasers · 3 sets · 2 tests") does not count picks.

## 9. Keyboard routes

| Action | Route |
|---|---|
| Pick bar, pick pick | one Tab stop; arrows move in the 2 x 2 grid; Space or Enter presses |
| Table | one Tab stop after the bar; arrows, Home, End, Ctrl+Home, Ctrl+End as built; Space or Enter marks or unmarks the pressed kind |
| From the table back to the bar | Shift+Tab |

## 10. Undo steps and texts

- Steps: "Mark pick", "Unmark pick" (test detail), "Use pick" (operation). "Choose best cell" is renamed to "Mark pick" (M3).
- Texts (each at most 70 characters): the tooltips "No Best set for this laser" to "No Second best set for this laser" (33), the state line of 5.2, the notices of 5.1, the status "<Kind>: <numbers>" and "<Kind> removed".

## 11. Decisions for veto (Proposal)

- **P1** The pick control is a 2 x 2 grid, not one row.
- **P2** The test detail's heading reads "Picks".
- **P3** The steps are "Mark pick" and "Unmark pick" for every kind.
- **P4** Several kinds in one table cell sit in fixed quadrants.
- **P5** The pick pick has no visible caption; its name "Use" is for screen readers and the step.
- **P6** Second best is the Best colour with a dashed frame.
- **P7** On the canvas, chips shrink to the first kind's glyph and then to nothing as the zoom falls; frames stay.
- **P8** Test-grid Engrave circle cells draw a 1px `--engrave-hatch` edge on screen.
- **P9** A pick line in the Materials tab is two lines.
- **P10** The pick bar's pressed kind is not kept between openings of a test.
- **P11** Fastest is petrol `#065774`, Deepest magenta `#B8008C`.
- **P12** Pressing the already pressed kind in the pick pick clears the typed numbers without arming.

## 12. Mismatches for the product owner, and what this revises

- **M1** Criterion 1 "four toggle buttons in one row": four symbol-and-word buttons need about 290px; the column is 228px (P1). Recommendation: amend to "in a 2 x 2 grid".
- **M2** Criterion 6 "the kind's initial or short word in the cell" and "symbols side by side": a cell is 24 or 17px wide with a 2px focus ring inside; the glyph is the mark in the cell, the word lives in the bar, tooltip, name and lines; several kinds sit in quadrants at 8px in 24px cells (P4), and narrow cells (9 to 12 columns) show the first kind's glyph only. Recommendation: amend.
- **M3** Criterion 2 keeps "Choose best cell" (`curvyo-ui-core/src/history_names.rs`) for all four kinds and lets the UX notes rename it: "Choose best cell" would read wrong in History for a Fastest mark, so the notes rename it to "Mark pick" for all kinds (P3); the internal key `judge_test` stays.
- **M4** Criterion 1 calls the four buttons "toggle buttons, exactly one pressed": built as `aria-pressed` buttons with manual activation (3.2), so arrows never write a step.
- **Revises `specs/0112-test-grid-and-materials/ux-notes.md`:** 8.2 item 5 (heading "Picks", the bar) and item 6 (pick lines); 8.3 (grid name "Picks of <id>", cell look, unmark, steps; G14 is replaced by criterion 3); 3.3 (the Engrave cell is a circle with the 7.2 edge); 11 ("Choose best cell" becomes "Mark pick", "Unmark pick"; "Use pick" added); section 2 row "Best cell frame" (`--accent` becomes `--pick-best`: the accent means selected, `0112` used it for want of a pick colour).
- Design system: the four tokens and the rows of "Material test picks (`0146`)" are added as a draft (this change); they become final when `0146` is Done.

## 13. Implementer checklist

- [ ] `PickControl`: 2 x 2, 112 x 28 cells, pressed fill plus semibold plus `aria-pressed`, dimmed `aria-disabled` focusable with tooltip and description, manual activation.
- [ ] Table: frames per 4.3 (dashed Second best), quadrants, tooltip, names, "Mark pick" / "Unmark pick", status texts, grid name and description.
- [ ] Pick lines under the table and in the Materials tab (two lines, read only).
- [ ] Operation: control under the Material pick, states with the kind, multi-row dimming and skip notice, "Use pick" clears overrides also on the pressed kind.
- [ ] Canvas overlay: selected grid only, frames with casing and the offset rule, chip size rule, never output or hit.
- [ ] Engrave test cells: the 1px `--engrave-hatch` edge, View: Operations only.
- [ ] Tokens in `frontend/src/index.css` with a comment pointing to `docs/design-system.md`; contrast asserted in a frontend test as `machineProfileButtons0130.test.ts` does.
- [ ] Review at 1x and 2x: the `SecondBestGlyph` "2" at 14px and up, the 8px quadrant glyphs at 12 columns, the 3 mm circle at 56 %, 100 % and 200 %.

## Links

Spec: `specs/0146-material-test-picks/specification.md`. ADR: `docs/adr/0093-material-test-picks-and-round-engrave-cells.md` (`specs/0146-material-test-picks/adrs.md`). Built notes revised: `specs/0112-test-grid-and-materials/ux-notes.md`, `specs/0028-material-test-library/ux-notes.md` 8.2 and 8.4. Design system: `docs/design-system.md` "Color tokens" (`--pick-*`) and "Material test picks (`0146`, draft 2026-11-26)".
