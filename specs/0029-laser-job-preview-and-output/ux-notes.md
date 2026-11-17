# 0029 Laser job preview and output: UX notes

ux-engineer, 2026-11-16, amended 2026-11-17. Written from `specification.md` (criteria 1 to 39; Fragen 219 C, 220 A, 221 B, 222 B, 223 A, 224 B with the homing request, 225 A, 226 A changed), `docs/design-system.md` ("Interaction conventions": no operating tips, no success notice except the export notice `0024` already has, hidden not disabled, no popups in the panel; rows "Kind chip places", "Canvas notice slot", "Inline delete confirm") and the built notes `specs/0101-laser-operations/ux-notes.md` (kind looks, `--cut`, `--engrave-hatch`, row and summary), `specs/0111-laser-operation-tools/ux-notes.md` (Operations view toggle, unfolded row, Motorised Z axis row), `specs/0028-material-test-library/ux-notes.md` 8.4 (live, stored, typed), `specs/0103-fixture-jobs/ux-notes.md` 3 (Fixture jobs section), `specs/0113-fixture-sets-and-pins/ux-notes.md`, `specs/0026-machine-profile/ux-notes.md` (Machine block, state row). Drafts: `docs/design-drafts/2026-11-laser-output/` screens 01 to 03. Customer rules applied: no invented colour (symbols and text; ink, the existing red and pale orange only), symbol-only buttons with the name as tooltip, units inside fields, no number field in the output, the one slider is the scrub slider, the bar is the Toolpath view's own, no operating tips, compact, nothing like LightBurn's layer table or preview window, no machine settings in operations. Items marked (Proposal) are vetoable at the demo; section 14 lists them as P1 to P32.

## Changes 2026-11-17 (customer feedback after trying PR #371)

| # | Feedback | Sections changed |
|---|---|---|
| 1 | Remove the Head position warning entirely | 6.3, 10, 13, 14 (P22 withdrawn), 15 |
| 2 | Play bar readout: time only, no metres | 5, 11, 13, 14 (P11 withdrawn) |
| 3 | No Output tab: the output sits in the Operations tab under the list, compact; every row gets its run number, time and numbers line; Own numbers only in the unfolded row | 0, 2, 4, 6 (rewritten), 7, 9, 11, 13, 14 (P3, P13, P15, P17, P18 replaced; P24 to P28, P30, P32 new), 15, 16 |
| 4 | Home first was hard to understand (why does it grey Start from?): one Start pick with three plain options, no greying | 6.2, 9, 10, 11, 12, 14 (P29), 15 |
| 5 | Air assist switch not operable in the Machine block | 8 (bug and fix), 9, 14 (P31), 16 |

Terminology: "run" is what one ring on the page marks (an operation that runs, or the fixture jobs of one kind), "check" a block or warn line, "ink" `--toolbar-icon`, "head" the drawn laser position of the play bar, "output group" the part of the Operations tab under the Fixture jobs section.

## 0. What is reused

| Part | Reused from (code) | Change in this slice |
|---|---|---|
| Page label toggle | `PageFrames.tsx` (the `ListTree` Operations view toggle, `0111` 5) | a second toggle `Route`; the pair is exclusive (P2) |
| Kind looks and glyphs | `OperationGlyph` (`OperationGlyphs.tsx`), the canvas looks of `0101` 13.2 | drawn along the job's moves instead of the items' outlines |
| Operation row | `OperationRow.tsx` (`0101` 4.2, `0111` 4.1) | line 2 gets the run ring and the time, a new line 3 the numbers (6.1) |
| Operations summary | `OperationsSection.tsx` (`0101` 7) | its count lines become checks, its first line goes to the total (6.4, P28) |
| Fixture jobs section | `FixtureJobsSection.tsx` (`0103` 3) | the fixture material pick and the fixture run line at the top of its body (6.2, P32) |
| Material pick | `MaterialPick.tsx` (inline list, `0028` 8.2) | "Fixture jobs run with", a kind filter (Score, Cut); the same inline list for the Start pick |
| Switch rows | `PanelSwitch`, `MachineBlock.tsx` | Air assist and the Output line in the Machine block; the state row keeps its place (8) |
| Notices | `CanvasNotice.tsx`, `useActionNotice`, the export notice and error dialog of `0024` criterion 4 | the export texts of section 10 |
| Windowing | `useScrollWindow` | row height 56 (64 coarse) instead of 40 (48) |

Not reused any more: the `output` strip entry and `OutputTab.tsx` (its parts move into the Operations tab), the Start from `SegmentGroup` and the Home first switch row.

## 1. Principle

The page is the preview. One toggle at the page name shows what the laser will do, in the order it will do it, with the same looks the operations already have; a bar under the page plays it. The Operations tab is where the job is made and checked: each operation row says its run number, its time and its numbers; under the list the output group says which machine, where the file starts, what stops the export and what only deserves a look, how long it takes, and then offers one button. Nothing in the output group is a number to type; a wrong number is fixed in its row a few pixels above, and every name in a check is a way there.

## 2. Tokens and symbols (no new colour)

| Use | Token |
|---|---|
| Cut and Cut-out burn moves, their arrowheads | `--cut` (as `0101`) |
| Score burn moves, arrowheads, rings, head, labels | `--toolbar-icon` |
| Engrave scan lines and arrowhead | `--engrave-hatch` (pale orange, 3.17:1 on `--canvas-bg`) |
| Travel moves | `--lane-line`, 1px, dashed 4 on 3 (3.45:1 on `--canvas-bg`) |
| Casing under rings, arrowheads, head | `--selection-casing` |
| After the head (scrubbed) | the same strokes at 35 % opacity and 60 % width (P7) |
| Block glyph | `--destructive` (4.8:1 on `--toolbar-bg`) |
| Warn glyph | ink; `--engrave-hatch` is 2.8:1 on `--toolbar-bg` and fails 3:1 for a glyph (P14) |
| Muted lines, row line 2 and 3, machine line | `--panel-muted-fg` |
| Row time, "typed" | ink |
| Pressed toggles, Export, slider fill and thumb ring | `--toolbar-icon-active-bg` / `--editor-accent` (existing) |
| Disabled Export | `--field-disabled-bg`, `--field-disabled-fg` |

| Control | Symbol (Lucide, 16px, 1.5px absolute stroke unless said) |
|---|---|
| Toolpath view | `Route` 14px in the 20px page-label button |
| Export | `Download` |
| Block / Warn | `OctagonX` / `TriangleAlert` |
| Play / Pause | `Play` / `Pause` |
| Start pick | `Crosshair` |
| Fixture material pick | `SwatchBook` |
| Run ring of the fixture jobs | the Fixtures tab glyph (`HolderGlyph`) 9px in the ring |
| Run kind | `OperationGlyph` |

## 3. Toolpath view toggle (criterion 8)

- A `button` in the canvas page label of each laser page, after the Operations view toggle, 4px gap: the look, size, hit area (24px), radius, hover and tooltip timing of `0111` 5. Name and tooltip "Toolpath view", `aria-pressed`. Default unpressed (P1).
- **Exclusive with Operations view** (P2): pressing Route shows ListTree unpressed (the page shows the job, not the operations); pressing ListTree while Route is pressed releases Route; releasing Route gives ListTree back the state it had. Both stay in place, so nothing moves under the pointer.
- Per laser page, session state; not saved, not a history step, no notice. No shortcut (as `0111` T6).
- Narrow frames: the rule of "Kind chip places"; both toggles stay as long as the label is drawn, Route goes with the label.
- While pressed, select, move and the edit tools work as on the drawn page (hit testing uses the items, not the moves); the selection box and handles draw over the toolpath.

## 4. The Toolpath look (criteria 8, 9, 11, 18, 37)

- Drawn instead of the items' own style: the job's burn moves, travel moves, rings and arrowheads. Fixtures and the grid stay as built. Items not assigned, operations with Output off, Unknown and damaged operations and Engrave images draw nothing. Hidden items that run and operations with Show off draw (criterion 11).
- **Burn:** Cut 1px solid `--cut`; Cut-out 2px dashed 6 on 3 `--cut`; Score 1.5px dotted ink; Engrave scan lines 1px `--engrave-hatch` along their real direction (Scan angle, Crosshatch). Engrave + Outline: its hatch as Engrave, its outline as Score (the sets it runs with).
- **Thinning** (P4): scan lines closer than 3 screen px are drawn every n-th line so the drawn spacing is 3 to 6px; zooming in shows every line. The lengths and times always count every line.
- **Travel:** 1px `--lane-line` dashed 4 on 3, from the origin to the first move, between paths and back to the origin at the end (criterion 25). Travel inside one Engrave area (the turn between two scan lines, overscan run-outs) is not drawn (P5).
- **Ring:** one per run, at the start of its first burn path, offset 12px up-left with a 1px ink tick to the point: 17px circle, white ground, 1.5px ink ring. Inside: for an operation the number of its row in the Operations tab (the digit of `0101` line 2), 10px semibold; for the fixture jobs of one kind the fixture glyph, 10px (P26). The rings are in run order on the page because the list is (Cuts last groups Marking before Cutting); a row that does not run leaves a gap in the numbers. Rings of later passes are not repeated.
- **Arrowhead:** an open 5px chevron in the move's stroke colour with a 3px white casing, at the start of each burn path, pointing along it; one per Engrave area (P4); left out when the path is shorter than 12 screen px (P6).
- Screen-constant sizes; rings, arrowheads and the head are `aria-hidden`, `pointer-events: none`.
- A change of the drawing, an operation, a material or the machine redraws in the same frame as the checks (criteria 3, 8).

## 5. The play bar (criterion 37)

- A floating chrome bar (`--toolbar-bg`, radius 8, `--panel-elevation-shadow`), 40px high, shown only while the active page's Toolpath view is pressed; one bar, for the active page. Placed 10px under the page frame, left-aligned to it, as wide as the frame between 360 and 640px; whenever the frame's bottom is out of view (below or above it) it sticks 12px above the canvas bottom (P8); it never goes under the tool columns (left edge at least `--rail-right` + 12).
- Left to right: **Play** (28px, symbol only, `Play` / `Pause`; name and tooltip "Play" / "Pause"), 10px, **the slider** (flexible, at least 120px; the `0028` slider look: 4px track ink at 35 %, accent fill, 14px white thumb with a 2px accent ring), 10px, **the readout** 12px tabular, **time only**: "6 min 6 s of 9 min 51 s" ("of" muted). No length (customer 2026-11-17); the bar stays one line at every width (P11 withdrawn).
- **Axis:** the slider runs over the job's time, so the time in the readout is exact at every position and slow moves take more of the slider (P9).
- **Start position:** the end, so the whole job shows at full weight when the view is turned on (P10). Moving the slider draws everything up to the head at full weight, the rest pale (section 2), and the **head**: a 16px ink ring, 2px, with a 4px white casing and a 2px ink dot.
- **Play:** from the slider's position, the whole job in 10 s; Pause stops where it is; at the end it stops at the end and the button shows Play; the next press starts at 0. Changing the slider while playing keeps playing from there. An edit while playing pauses (the job changed). Reduced motion does not stop Play (it is the content the maker asked for) (P12).
- **Keys:** the slider is one Tab stop (`role="slider"`, name "Run position", `aria-valuetext` = the readout): Left and Right 1 % of the run, Page Up and Page Down 10 %, Home and End. Space or Enter on Play. Tab order: page label toggles, then the bar.
- Session state; no history step, no notice.

## 6. The Operations tab with the output (criteria 1 to 7, 10, 38)

No Output tab and no `FileOutput` strip entry (P24). The Operations tab of a laser page reads, top to bottom: header, Cuts last, the add group (`0101`), the list (Marking, Cutting), the Settings block of a multi-selection (`0028` M10), the **Fixture jobs section** (`0103` 3, with 6.2), the **output group** (6.3 to 6.5). One 256px column; the list keeps its windowing; the tab scrolls as one. Draft 02.

### 6.1 Operation row (criteria 2, 3, 10; P25 to P27)

The built row of `0101` 4.2 and `0111` 4.1 (line 1 glyph, name, Show, Output, Trash, Up, Down; unfolded body under line 3) with two changes. Row height 56px (64 coarse); Engrave + Outline 70px (78).

| Line | Content |
|---|---|
| 1 (26px) | unchanged |
| 2 (16px, 11px muted) | a row that runs: the **run ring** (14px circle, 1.25px ink ring, white ground, the row's digit 9px semibold ink; the ring of section 4), 4px, "Cut · 6 items" (kind, count, " · Group 3 in part" as built); at the right the run's time in ink, tabular ("3 min 21 s"; "–" with No settings). A row that does not run (Output off, no item, Unknown, damaged settings) keeps the plain digit as built ("4 · Cut · 2 items · Output off") and has no time |
| 3 (14px, 11px muted, tabular) | the effective numbers: "35 %, 6,000 mm/min, 1 ×, 0.25 mm" (Interval for Engrave kinds); a typed number is followed by the word "typed" in ink ("90 % typed, 600 mm/min, 2 ×"); no settings: "No settings". Shown on every row, folded or unfolded, Output on or off; Unknown rows: none |
| 4 (14px, Engrave + Outline only) | "Outline " and the numbers of its outline set |

- The material name, "Own numbers" and the live or stored state show only in the unfolded row (its Material pick, `0028` 8.4), never on a folded row (customer 2026-11-17). A stored or missing material is still named by its check (6.4).
- Line 2 and line 3 are part of the row's press area (a press selects and unfolds, as line 2 today); no new Tab stop. The row's accessible name adds the run and time: "Cut 1, run 3, 3 min 21 s" (6.6).
- The numbers are the same as the unfolded row's fields (criterion 3), recomputed in the same frame.

### 6.2 Fixture jobs section (criterion 2, Frage 221 B; P32)

- Body, top, shown only while a fixture job has Output on (P17): the pick **Fixture jobs run with** (`MaterialPick`, full width x 28, `SwatchBook`, label inside left, the material at the right, inline list under it, no popup). Its list shows the library; a material without a set of a needed kind for this laser is listed muted with "No Score set" and is not pickable (as `0028` 8.2). A stored pick stays when it hides.
- Under it, one **fixture run line** per kind that runs: line a (16px, 11px muted) the run ring with the fixture glyph, 4px, the kind word ("Score"), the time at the right in ink; line b (14px) the numbers of that material's set ("20 %, 3,000 mm/min, 1 ×"; "No settings" without a pick). Static, no Tab stop.
- Then the job rows as built (`0103`, `0113` 6.3).
- The collapsed header "Fixture jobs (2)" shows the fixture runs' total time at its right (11px muted, tabular), nothing when none runs.

### 6.3 The output group (P24, P30, P29)

After a 1px divider, the group heading "Output" (11px semibold muted, the `0113` heading), then:

1. **Machine line** (12px, ink name semibold, the rest muted, wraps): "Diode 10 W · GRBL", plus " · air assist" when Air assist is on (P30). Work area, origin corner, laser mode and the S range stay in the Document tab's Machine block. Text, no stop.
2. **Start pick** (P29, replaces Start from and Home first): `MaterialPick` look, full width x 28, `Crosshair`, label "Start" inside left, the value at the right, `ChevronDown`; Enter, Space or a press opens the inline list under it (no popup): **Machine origin** (default), **Head position**, **Home, then machine origin**, the current one with `Check` and the selected ground. Plain words, no greying, no explanation text; the button's tooltip is its name "Start". A pick closes the list, writes one undo step and recomputes the checks and the total in the same frame.
3. **Checks** (6.4), after a divider; absent when there is none.
4. **Total** (6.5), after a divider; absent when nothing runs.
5. **Export** (6.6).

| Start | File (criteria 24, 26, 38) |
|---|---|
| Machine origin | absolute moves from the machine origin, no homing |
| Head position | relative moves (GRBL `G91`) from where the head stands; the bed check of criterion 4 b is measured from the job's own box; no warning (customer 2026-11-17) |
| Home, then machine origin | the format's home command first (GRBL `$H`), then as Machine origin |

### 6.4 Checks (criteria 4 to 7, 33, 38; P28)

- `role="list"` named "Checks"; each line a `listitem`: 16px glyph (`OctagonX` red for a block, `TriangleAlert` ink for a warn), 6px, text 12px ink, wrapping, 2px vertical padding. Screen readers hear "Blocks: " or "Warning: " before the text (sr-only).
- The `0101` summary's count lines ("2 items not assigned." and the others) are these checks now, with the check texts of section 10; there is one list of findings in the tab, not two (P28).
- The name or count that leads the text is a text button, underlined (no colour): items → selects them on the canvas (P15); an operation → selects and unfolds its row in the list above, scrolls it into view, focus on its first field (same tab, P15); "Fixture jobs" → expands the Fixture jobs section and focuses the pick. Each is a Tab stop in list order.
- Nothing runs (criterion 7): the block line "Nothing runs. Assign items to an operation.", no total.

### 6.5 Total (criterion 10)

"About 9 min 51 s" 13px semibold; under it, 12px muted: "Runs 3 operations, 16 items." (the first line of the `0101` summary, its words unchanged; "16 items" an underlined text button, as the checks' buttons, that selects the running items; only while a work operation runs), "1 fixture job runs." (the `0103` criterion 17 line, only while a fixture job runs; the count "1 fixture job" an underlined text button, as the checks' buttons, that selects those jobs' fixtures; amended 2026-11-17, Proposal) and "Burn 38.29 m · travel 2.26 m" (criterion 10 formats). `role="status"`, `aria-atomic="true"` as the summary today. Runs with no settings count nothing.

### 6.6 Export (criteria 1, 4, 27)

- A Primary button, full width, 32px high: `Download` 16px and "Export job file" 13px (P16). No tooltip while enabled (the words name it); the name is "Export job file".
- Blocked: the disabled look, `aria-disabled="true"` (it stays a Tab stop), tooltip (opens above, `side="top"`) and accessible description the first block's text. A press does nothing.
- Press: the OS save dialog (browser: a download) with "<project> - <page>.<extension>", the extension and the dialog's file type from the output format (GRBL: `.gcode`, "GRBL G-code"); after a write, section 7.

## 7. After Export (criteria 27 to 30)

- One canvas notice, `role="status"`: "Exported Coasters - Laser 1.gcode." (browser "Downloaded …"), 3 s; when fixture jobs were switched off a second line "Output turned off on 2 fixture jobs." and 5 s (P19). Name cut with an ellipsis to keep each line at 70 characters.
- One history step "Export job file" (fixture jobs off, stored numbers refreshed, Lasered). The fixture run line and pick leave the Fixture jobs section and the fixture rings leave the page; the operation rings keep their numbers (a ring is its row's digit).
- Cancel: nothing, no step, no notice. Write failure: the existing error dialog with the system's reason.

## 8. Air assist and Output in the Machine block (criteria 25, 32, 39; P21, P23, P31)

- A row in the Machine block of the Document tab after "Motorised Z axis" (`0111` 6.3): label "Air assist", `PanelSwitch` at the right edge of the row (`role="switch"`, no on/off text), default off; a click on the words flips it too. One undo step "Air assist on" / "Air assist off" on the page's profile copy; Differs, Save as profile and Update profile include it like Laser mode. Nothing in the operations (P21).
- **Bug found 2026-11-17 (customer: "not operable"), reproduced in the dev build:** on a copy that equals its profile, the first press turns Air assist on and the state row "Differs from profile" (`0026` §2) appears above Work area; every row below moves down 36px, so a second press at the same point flips Motorised Z axis (its label), not Air assist. To the maker the switch cannot be turned off. The same holds for Laser mode, Motorised Z axis and the origin corner. The browser check `frontend/tests/browser/laser-output.cjs` misses it because it finds the switch by role before each press.
- **Fix (P31):** nothing in the Machine block moves under the pointer. The state row keeps its 28px place on every laser page, empty while the copy equals its profile (no text, no button, `aria-hidden`), "Differs from profile" with its buttons otherwise; "Not in your profiles" uses the same place. Where: `frontend/src/components/workbench/MachineBlock.tsx`, the conditional state row above "Work area". The switch rows (Laser mode, Motorised Z axis, Air assist) put the switch at the row's right edge, label at the left (the `0111` 6.3 rule; today the switch sits right after the words because the `PanelSwitch` label is the row's only flex child). Test: a browser check that presses twice at the same screen point and reads Air assist on, then off, and Motorised Z axis unchanged (draft 03 A).
- **Output** (P23): the row after Air assist, read-only while there is one output format: "Output" 13px at the left, "GRBL" 13px muted at the right, plain text, no control, no Tab stop, no step. With a second format it becomes a `SegmentGroup` (up to three formats, else the `MaterialPick`-style inline list) in the same row, one undo step "Set output format", included in Differs, Save as profile and Update profile; that pick is built with the second format, not now. A damaged or unknown value reads "Output: unknown" and the export blocks (criterion 4 c).

## 9. Keyboard routes

| Action | Route |
|---|---|
| Toolpath view | Tab to the toggle in the page label, Space or Enter |
| Play bar | Tab to Play (Space or Enter), Tab to the slider (arrows 1 %, Page Up and Down 10 %, Home, End) |
| Operations tab, after the list | Tab: the Fixture jobs header, the fixture pick (Enter opens the inline list; arrows, Enter picks, Escape closes), the job rows, the Start pick (same keys), each check's button, Export |
| Check button | Enter or Space selects or goes there |
| Air assist | Tab to the switch, Space |

## 10. Texts (each at most 70 characters)

| Text | Kind |
|---|---|
| "Cut 2 has no power and speed." / "has no power." / "has no speed." | block (4 a) |
| "Fixture jobs have no material. Pick one above." / "Fixture jobs: Oak 4 mm has no Score set." | block (4 a) |
| "3 items lie outside the 400 × 300 mm bed." | block (4 b) |
| "The machine profile of this page is damaged." | block (4 c) |
| "Nothing runs. Assign items to an operation." | block (4 d, 7) |
| "The machine is Generic laser; its limits may not match yours." | warn |
| "Score 1: Oak 4 mm is not found, using stored numbers." | warn |
| "Score 1: Oak 4 mm has no Score set, using stored numbers." | warn |
| "Cut 1 runs a typed power, not Birch 3 mm’s." (power, speed, passes listed: "typed power and speed") | warn |
| "Engrave 1 runs at 6,000 mm/min, the machine's maximum." | warn (21) |
| "Cut 1 has a Z offset; the machine has no motorised Z axis." | warn (19) |
| "2 items are not assigned." / "1 item runs while hidden." / "Score 2 has Output off." ("2 operations have Output off.") | warn |
| "Cut-out 1: 1 item has no closed contour." | warn |
| "2 operations have damaged settings and do not run." | warn (33) |
| "Runs 3 operations, 16 items." | total, line 2 (`0101` summary words) |
| "1 fixture job runs." / "2 fixture jobs run." | total, after the work line (`0103` criterion 17 words) |
| "Machine origin" / "Head position" / "Home, then machine origin" | Start options |
| "Damaged" | the Start pick's value while the stored Start is damaged (criterion 4 e; Proposal) |
| "Exported <name>." / "Downloaded <name>." / "Output turned off on N fixture jobs." | canvas notice |

Removed 2026-11-17: "Starts where the head stands; the bed is not checked." and the tooltip "Home first starts from the machine origin.".

## 11. Accessible names

| Element | Name | State or description |
|---|---|---|
| Toolpath view | "Toolpath view" | `aria-pressed` |
| Play | "Play" / "Pause" | |
| Slider | "Run position" | `aria-valuetext` the readout ("6 min 6 s of 9 min 51 s") |
| Operation row name button | "Cut 1, run 3, 3 min 21 s"; a row that does not run as built | `aria-expanded` as built |
| Fixture pick | "Fixture jobs run with" | value the material |
| Start pick | "Start" | value the option; the list a `radiogroup` "Start" |
| Air assist | "Air assist" (switch) | `aria-checked` |
| Checks | "Checks" (list); lines with the sr-only "Blocks:" / "Warning:" | |
| Total | "Total" (status) | |
| Export | "Export job file" | `aria-disabled`, description the first block |
| Output line | "Output: GRBL" (plain text) | |

## 12. Undo steps

"Export job file", "Pick fixture material", "Start: Machine origin" / "Start: Head position" / "Start: Home, then machine origin", "Air assist on" / "Air assist off". No step for the Toolpath view, the slider, Play, a check button, opening a pick or the save dialog cancelled.

## 13. Empty, error, mixed and large states

- **Nothing runs:** 6.4; Export disabled; no total; the rows keep their plain digits and no time; the Toolpath view draws only the fixtures and grid, the bar shows "0 s of 0 s" with Play and the slider disabled (P20).
- **Laser (off), other page kinds:** no output group, no run ring, time or line 3 on the rows, no Route toggle.
- **Generic laser:** the warn; the machine line names it.
- **Damaged profile:** block (c); the machine line reads "Machine profile damaged".
- **Large:** 50 operations: the list keeps its windowing at 56px rows; 1,000 items: arrowheads skipped under 12 px, at most 2,000 per frame, rings always; the budgets of criterion 36 (pan, zoom and slider within 16 ms, the job built lazily, only cut at a length for the slider).

## 14. Decisions for veto (Proposal)

- **P1** Route toggle after the Operations view toggle, same look, unpressed by default.
- **P2** The two page-label toggles are exclusive; releasing Route restores ListTree.
- **P3** Replaced by P26 (2026-11-17).
- **P4** Engrave scan lines thinned to 3 to 6 screen px; one arrowhead per Engrave area.
- **P5** Travel inside one Engrave area is not drawn (counted).
- **P6** No arrowhead on a path shorter than 12 screen px; at most 2,000 per frame.
- **P7** After the head: 35 % opacity, 60 % width; head = ink ring with casing and dot.
- **P8** Play bar 10 px under the page frame, sticky at the canvas bottom, only while the Toolpath view is pressed.
- **P9** The slider's axis is the job's time.
- **P10** The slider starts at the end; Page Up and Down step 10 %.
- **P11** Withdrawn 2026-11-17: the time-only readout never wraps.
- **P12** An edit while playing pauses; reduced motion does not stop Play.
- **P13** Replaced by P24.
- **P14** Block glyph in `--destructive`, warn glyph in ink.
- **P15** Check buttons: items select on the canvas; an operation selects, unfolds and scrolls to its row in the same tab; Fixture jobs opens its section and focuses the pick. The Output off line is a count line of the `0101` summary and selects its items (M11).
- **P16** Export as a full-width Primary with symbol and words, `aria-disabled` so the block stays reachable.
- **P17** The fixture pick shows only while a fixture job has Output on.
- **P18** Replaced by P30.
- **P19** One export notice with a second line for the fixture jobs, 3 s or 5 s.
- **P20** Nothing runs: the bar's controls are disabled, not hidden (the bar keeps its place).
- **P21** Air assist is a switch row after Motorised Z axis, one undo step.
- **P22** Withdrawn 2026-11-17: no Head position warning.
- **P23** The output format is a read-only "Output: GRBL" row after Air assist; it becomes a pick only when a second format exists.
- **P24** No Output tab: the output group (machine line, Start, checks, total, Export) sits in the Operations tab after the Fixture jobs section, in that order.
- **P25** Operation rows 56 px (64 coarse): line 2 run ring, kind, count and the time at the right; line 3 the effective numbers with "typed"; Engrave + Outline a line 4 "Outline …".
- **P26** The ring number is the row's digit (the `0101` digit route), so a ring on the page names its row and the digit stays visible; rows that do not run keep the plain digit; the fixture jobs' ring shows the fixture glyph. Alternative if vetoed: a separate run count 1 to n with the fixture jobs first, shown beside the digit.
- **P27** Material, Own numbers and live or stored only in the unfolded row; folded rows show numbers only.
- **P28** The `0101` summary's count lines become checks with the check texts; its first line "Runs N operations, M items." is line 2 of the total.
- **P29** Start is one pick with three plain options (Machine origin, Head position, Home, then machine origin), the inline list of the material pick, `Crosshair`, default Machine origin; nothing greys out.
- **P30** One machine line "<profile> · <format>" plus " · air assist"; the rest of the machine stays in the Document tab.
- **P31** The Machine block's state row keeps its place (empty when in sync) and switches sit at the row's right edge, so nothing moves under the pointer.
- **P32** The fixture pick and the fixture run lines sit at the top of the Fixture jobs section body; its header shows the fixture runs' time.

## 15. Mismatches for the product owner and the architect, design-system changes

Still open from 2026-11-16: **M2** (criterion 37's test assumes one feed; with P9 it reads "half the slider reads half the total time, and the drawn lengths up to the head equal the file's up to that move"), **M6** (criterion 2 now states Air assist; kept by P30), **M7** (one notice with two lines, P19), **M8** withdrawn (the drafts are redrawn). **M1, M3, M4** are replaced by M9 to M16; **M5** is narrowed: only Export is disabled, not hidden.

- **M9 (criterion 1, Words "Output tab", "Gate"):** no Output tab; the Operations tab of a laser page holds, after the Fixture jobs section, the output group: machine line, Start pick, checks, total, Export. Its only controls stay the fixture pick, the Start pick and the scrub slider (on the canvas).
- **M10 (criterion 2):** the per-operation gate lines become the rows' line 2 and 3 (6.1): run ring = the row's digit (P26), the time at the right of line 2, the numbers with "typed" on line 3; the material name and the live/stored word only in the unfolded row (P27). The fixture jobs: the pick and one run line per kind at the top of the Fixture jobs section, ring with the fixture glyph (P32). The machine line is "<profile> · <format>[ · air assist]" (P30); work area, origin and the laser-mode line are in the Machine block.
- **M11 (criteria 4, 5, 7 and `0101` criterion 17):** a check's operation button selects and unfolds its row in the same tab; the `0101` summary's count lines merge into the checks and its "Runs …" line into the total (P28). A change of accepted `0101` behaviour (the summary's place and words of its count lines); default: merge, as the customer asked for one compact tab. The select buttons stay as accepted (lead decision, 2026-11-17): "2 items are not assigned." and "1 item runs while hidden." select their items, "Score 2 has Output off." / "2 operations have Output off." select the items in those operations (not the operations), and "16 items" of "Runs 3 operations, 16 items." selects the running items.
- **M12 (criterion 10, 36 e):** the summary is the total of the output group with the "Runs …" line; 36 e reads "the Operations tab with the output group opens within 100 ms".
- **M13 (criterion 37):** the readout is "<elapsed> of <total>", no length; the test's length check stays on the view's lengths, not on the readout.
- **M14 (criteria 24, 26, 32, 38):** one Start pick with three values replaces the two switches; no disabled state, no tooltip, no Head position warning (the 53-character text is removed); "Home, then machine origin" writes the home command first; Head position writes relative moves; the bed check stays measured from the job's own box. The two page registers `x.curvyo.laser.start_from` and `.home_first` become one register `x.curvyo.laser.start` with `machine_origin` (default, also when absent), `head_position`, `home_then_origin`; format 48 is unreleased, so its number stays and no migration is needed (architect: ADR 0065 §2 and §4, an unknown value's reading). The golden files `home_first.gcode` and `start_head_position.gcode` keep their bytes.
- **M15 (criterion 28 a):** after Export the operation rings keep their numbers (no "numbers move up").
- **M16 (criterion 39 and `0026`):** the Machine block's state row keeps its place on every laser page (P31); a UX change to accepted `0026` notes §2, no spec criterion changes. The Air assist bug fix is the implementer's (section 8).
- Design-system rows to add at acceptance: "Toolpath view toggle (`0029`)" in "Kind chip places", "Toolpath look (`0029`)", "Play bar (`0029`)" (the one slider of the app), "Operation row line 3 and run ring (`0029`)", "Output group (`0029`)", "Check line (`0029`)", "Start pick (`0029`)", "Air assist row" and "Output row" in the Machine block, and in "Interaction conventions": "Nothing moves under the pointer after a press (the state row keeps its place)". No token.

## 16. Implementer checklist

- Page label: Route toggle after ListTree, exclusive pair, session state.
- Toolpath drawing from the export's move builder: kind looks, thinning, travel, rings (row digit; fixture glyph for fixture jobs), arrowheads, pale after the head, head ring; redraw live.
- Play bar: placement and sticky rule, Play/Pause, time-axis slider with keys, time-only readout on one line.
- Remove the `output` strip entry; move the output parts into the Operations tab after the Fixture jobs section (machine line, Start pick, checks, total, Export).
- Operation row: 56px (64 coarse; 70 and 78 for Engrave + Outline), line 2 run ring and time, line 3 numbers with "typed", accessible name with run and time; windowing and the reorder ghost at the new height.
- Fixture jobs section: the pick and the fixture run lines at the top of the body, shown while a fixture job has Output on; header time.
- Checks: absorb the `0101` summary's count lines; the operation button selects, unfolds and scrolls in the same tab; "Fixture jobs" expands the section and focuses the pick. Total with the "Runs …" line.
- Start pick: one pick, three options, one undo step "Start: <option>", one page register; remove the Start from segment, the Home first switch, their disabled state and tooltip, and the Head position warning.
- Export flow: file name and file type from the output format, dialog, notice texts, one step "Export job file", error dialog, cancel without step.
- Machine block: the state row keeps its 28px place; switches at the row's right edge; Air assist switch row and its step; the read-only Output row; a browser check pressing twice at one point.
- Texts of section 10 at 70 characters at most; no new colour, no popup, no operating tip; budgets of criterion 36.

## Links

Spec: `specs/0029-laser-job-preview-and-output/specification.md`. Drafts: `docs/design-drafts/2026-11-laser-output/` (01 to 03). Built notes: `specs/0101-laser-operations/ux-notes.md`, `specs/0111-laser-operation-tools/ux-notes.md`, `specs/0028-material-test-library/ux-notes.md`, `specs/0103-fixture-jobs/ux-notes.md`, `specs/0113-fixture-sets-and-pins/ux-notes.md`, `specs/0026-machine-profile/ux-notes.md`.
