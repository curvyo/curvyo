# 0029 Laser job preview and output: UX notes

ux-engineer, 2026-11-16. Written from `specification.md` (criteria 1 to 38; Fragen 219 C, 220 A, 221 B, 222 B, 223 A, 224 B with the homing request, 225 A, 226 A changed), `docs/design-system.md` ("Interaction conventions": no operating tips, no success notice except the export notice `0024` already has, hidden not disabled, no popups in the panel; rows "Kind chip places", "Canvas notice slot", "Inline delete confirm") and the built notes `specs/0101-laser-operations/ux-notes.md` (kind looks, `--cut`, `--engrave-hatch`), `specs/0111-laser-operation-tools/ux-notes.md` (Operations view toggle, unfolded row, Motorised Z axis row), `specs/0028-material-test-library/ux-notes.md` 8.4 (live, stored, typed), `specs/0113-fixture-sets-and-pins/ux-notes.md`. Drafts: `docs/design-drafts/2026-11-laser-output/` screens 01 to 03. Customer rules applied: no invented colour (symbols and text; ink, the existing red and pale orange only), symbol-only buttons with the name as tooltip, units inside fields, no number field in the tab, the one slider is the scrub slider, the bar is the Toolpath view's own, nothing like LightBurn's layer table or preview window, no machine settings in operations. Items marked (Proposal) are vetoable at the demo; section 14 lists them as P1 to P22.

Terminology: "run" is one line of the Runs list (an operation, or the fixture jobs of one kind), "check" a block or warn line, "ink" `--toolbar-icon`, "head" the drawn laser position of the play bar.

## 0. What is reused

| Part | Reused from (code) | Change in this slice |
|---|---|---|
| Page label toggle | `PageFrames.tsx` (the `ListTree` Operations view toggle, `0111` 5) | a second toggle `Route`; the pair is exclusive (P2) |
| Kind looks and glyphs | `OperationGlyph` (`OperationGlyphs.tsx`), the canvas looks of `0101` 13.2 | drawn along the job's moves instead of the items' outlines |
| Tab strip | `PanelStrip.tsx` | `output` after `fixtures`, `FileOutput` |
| Material pick | `MaterialPick.tsx` (inline list, `0028` 8.2) | "Fixture jobs run with", a kind filter (Score, Cut) |
| Segmented control | `SegmentGroup.tsx` | Start from |
| Switch rows | `PanelSwitch`, `MachineBlock.tsx` | Home first; Air assist in the Machine block |
| Notices | `CanvasNotice.tsx`, `useActionNotice`, the export notice and error dialog of `0024` criterion 4 | the G-code texts of section 10 |
| Windowing | `useScrollWindow` | the Runs list above 20 runs |

## 1. Principle

The page is the preview. One toggle at the page name shows what the laser will do, in the order it will do it, with the same looks the operations already have; a bar under the page plays it. The Output tab says, top to bottom, which machine, how the file starts, what runs with which numbers and from where, what stops the export and what only deserves a look, how long it takes, and then offers one button. Nothing in the tab is a number to type; a wrong number is fixed where it lives, and every name in a check is a way there.

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
| Muted lines, source words, line 2 and 3 of a run | `--panel-muted-fg` |
| Pressed toggles, Export, slider fill and thumb ring | `--toolbar-icon-active-bg` / `--editor-accent` (existing) |
| Disabled Export | `--field-disabled-bg`, `--field-disabled-fg` |

| Control | Symbol (Lucide, 16px, 1.5px absolute stroke unless said) |
|---|---|
| Toolpath view | `Route` 14px in the 20px page-label button |
| Output tab | `FileOutput` |
| Export | `Download` |
| Block / Warn | `OctagonX` / `TriangleAlert` |
| Play / Pause | `Play` / `Pause` |
| Home first | `House` (the spec's `Home`) |
| Live / Stored source | `Link2` / `Link2Off` 12px (the `0028` states) |
| Fixture material pick | `SwatchBook` |
| Run kind | `OperationGlyph`; the fixture jobs line the Fixtures tab glyph |

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
- **Ring:** one per run, at the start of its first burn path, offset 12px up-left with a 1px ink tick to the point: 17px circle, white ground, 1.5px ink ring, the run number 10px semibold (P3: the number of the run in the Output tab; the fixture jobs come first). Rings of later passes are not repeated.
- **Arrowhead:** an open 5px chevron in the move's stroke colour with a 3px white casing, at the start of each burn path, pointing along it; one per Engrave area (P4); left out when the path is shorter than 12 screen px (P6).
- Screen-constant sizes; rings, arrowheads and the head are `aria-hidden`, `pointer-events: none`.
- A change of the drawing, an operation, a material or the machine redraws in the same frame as the gate (criteria 3, 8).

## 5. The play bar (criterion 37)

- A floating chrome bar (`--toolbar-bg`, radius 8, `--panel-elevation-shadow`), 40px high, shown only while the active page's Toolpath view is pressed; one bar, for the active page. Placed 10px under the page frame, left-aligned to it, as wide as the frame between 360 and 640px; when the frame's bottom leaves the view it sticks 12px above the canvas bottom (P8).
- Left to right: **Play** (28px, symbol only, `Play` / `Pause`; name and tooltip "Play" / "Pause"), 10px, **the slider** (flexible, at least 120px; the `0028` slider look: 4px track ink at 35 %, accent fill, 14px white thumb with a 2px accent ring), 10px, **the readout** 12px tabular: "6 min 6 s of 9 min 51 s · 36.55 m of 40.55 m" ("of" and "·" muted). Below 480px of bar width the readout takes a second line under the slider (P11).
- **Axis:** the slider runs over the job's time, so the time in the readout is exact at every position and slow moves take more of the slider (P9). Length follows from the same position.
- **Start position:** the end, so the whole job shows at full weight when the view is turned on (P10). Moving the slider draws everything up to the head at full weight, the rest pale (section 2), and the **head**: a 16px ink ring, 2px, with a 4px white casing and a 2px ink dot.
- **Play:** from the slider's position, the whole job in 10 s; Pause stops where it is; at the end it stops at the end and the button shows Play; the next press starts at 0. Changing the slider while playing keeps playing from there. An edit while playing pauses (the job changed). Reduced motion does not stop Play (it is the content the maker asked for) (P12).
- **Keys:** the slider is one Tab stop (`role="slider"`, name "Run position", `aria-valuetext` = the readout): Left and Right 1 % of the run, Page Up and Page Down 10 %, Home and End. Space or Enter on Play. Tab order: page label toggles, then the bar.
- Session state; no history step, no notice.

## 6. The Output tab (criteria 1 to 7, 10, 38)

Strip: after Fixtures, `FileOutput`, name and tooltip "Output", laser pages with the extension on only. Header "Output" and the Laser chip, no subject line. One 244px column, scrolls as one; groups with the 11px semibold muted heading of `0113` 3.

### 6.1 Order (P13)

1. **Machine:** line 1 12px ink, the profile name semibold: "Diode 10 W · 400 × 300 mm · origin bottom-left · GRBL" (wraps); line 2 muted: "Dynamic power (M4) · S 0 to 1000", plus " · air assist" when Air assist is on (P18). Text, no stop.
2. **Start:** the Start from `SegmentGroup` full width, "Machine origin" | "Head position" (`role="radiogroup"` "Start from", one Tab stop, arrows move and pick); under it the **Home first** row: `House` 16px and "Home first" 13px at the left, `PanelSwitch` at the right. With Home first on, Start from shows Machine origin, greyed (`aria-disabled`), tooltip "Home first starts from the machine origin." (the spec asks for disabled here; M5).
3. **Runs:** heading "Runs" with "4 in this order" at the right (muted, tabular). The pick **Fixture jobs run with** (`MaterialPick`, 244 x 28, `SwatchBook`, label inside left, the material at the right, inline list under it, no popup), shown only while a fixture job has Output on (P17; a stored pick stays). Its list shows the library; a material without a set of a needed kind for this laser is listed muted with "No Score set" and is not pickable (as `0028` 8.2). Then one **run row** per run (6.2).
4. **Checks** (after a 1px divider): blocks, then warns, in the order of criteria 4 and 5 (6.3). Absent when there is none.
5. **Total** (after a divider): "About 9 min 51 s" 13px semibold, under it "Burn 38.29 m · travel 2.26 m" 12px muted (criterion 10 formats). Runs with no settings count nothing and show "–". Absent when nothing runs.
6. **Export** (6.4).

### 6.2 Run row (static, no Tab stop; criterion 2)

| Line | Content |
|---|---|
| 1 (22px) | run number in a 16px ink ring (the ring of section 4), 4px, kind glyph, 4px, name 13px ("Engrave 1"; "Fixture jobs · Score"), the run's time at the right 12px tabular ("6 min 4 s", "–" without settings) |
| 2 (14px, 11px muted, indent 40px) | material name or "Own numbers" or "No material", then " · " and the source: `Link2` "live" or `Link2Off` "stored" |
| 3 (14px) | "35 %, 6,000 mm/min, 1 ×, 0.25 mm" (Interval for Engrave kinds; Engrave + Outline adds a second numbers line for its outline set); a typed number is followed by the word "typed" in ink ("90 % typed"); no settings: "No settings" |

The numbers are the effective ones, the same as the operation's unfolded row (criterion 3). Unknown or damaged operations get no row (criterion 33).

### 6.3 Checks (criteria 4 to 7, 33, 38)

- `role="list"` named "Checks"; each line a `listitem`: 16px glyph (`OctagonX` red for a block, `TriangleAlert` ink for a warn), 6px, text 12px ink, wrapping, 2px vertical padding. Screen readers hear "Blocks: " or "Warning: " before the text (sr-only).
- The name or count that leads the text is a text button, underlined (no colour): items → selects them on the canvas, the tab stays (P15); an operation → switches to the Operations tab with that row selected and unfolded, focus on its first field; "Fixture jobs" → focus to the fixture pick. Each is a Tab stop in list order.
- Texts in section 10. Nothing runs (criterion 7): the Runs list is replaced by the block line "Nothing runs. Assign items to an operation.", no total.

### 6.4 Export (criteria 1, 4, 27)

- A Primary button, full width, 32px high: `Download` 16px and "Export G-code" 13px (P16: the one action of the tab gets its words). No tooltip while enabled (the words name it); the name is "Export G-code".
- Blocked: the disabled look, `aria-disabled="true"` (it stays a Tab stop), tooltip and accessible description the first block's text. A press does nothing.
- Press: the OS save dialog (browser: a download) with "<project> - <page>.gcode"; after a write, section 7.

## 7. After Export (criteria 27 to 30)

- One canvas notice, `role="status"`: "Exported Coasters - Laser 1.gcode." (browser "Downloaded …"), 3 s; when fixture jobs were switched off a second line "Output turned off on 2 fixture jobs." and 5 s (P19). Name cut with an ellipsis to keep each line at 70 characters.
- One history step "Export G-code" (fixture jobs off, stored numbers refreshed, Lasered). The fixture jobs leave the Runs list and the toolpath; the numbers of the other runs move up.
- Cancel: nothing, no step, no notice. Write failure: the existing error dialog with the system's reason.

## 8. Air assist in the Machine block (criteria 25, 32)

- A row in the Machine block of the Document tab after "Motorised Z axis" (`0111` 6.3): label "Air assist", `PanelSwitch` at the right (`role="switch"`, no on/off text), default off. One undo step "Air assist on" / "Air assist off" on the page's profile copy; Differs, Save as profile and Update profile include it like Laser mode. Nothing in the operations (P21).

## 9. Keyboard routes

| Action | Route |
|---|---|
| Toolpath view | Tab to the toggle in the page label, Space or Enter |
| Play bar | Tab to Play (Space or Enter), Tab to the slider (arrows 1 %, Page Up and Down 10 %, Home, End) |
| Output tab | Tab: Start from (one stop, arrows), Home first, the fixture pick (Enter opens the inline list; arrows, Enter picks, Escape closes), each check's button, Export |
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
| "Starts where the head stands; the bed is not checked." | warn (38, P22) |
| "Home first starts from the machine origin." | tooltip |
| "Exported <name>." / "Downloaded <name>." / "Output turned off on N fixture jobs." | canvas notice |

## 11. Accessible names

| Element | Name | State or description |
|---|---|---|
| Toolpath view | "Toolpath view" | `aria-pressed` |
| Play | "Play" / "Pause" | |
| Slider | "Run position" | `aria-valuetext` the readout |
| Output tab | "Output" | |
| Start from | "Start from" (radiogroup) | `aria-disabled` with Home first |
| Home first, Air assist | "Home first", "Air assist" (switch) | `aria-checked` |
| Fixture pick | "Fixture jobs run with" | value the material |
| Checks | "Checks" (list); lines with the sr-only "Blocks:" / "Warning:" | |
| Export | "Export G-code" | `aria-disabled`, description the first block |

## 12. Undo steps

"Export G-code", "Pick fixture material", "Start from machine origin" / "Start from head position", "Home first on" / "Home first off", "Air assist on" / "Air assist off". No step for the Toolpath view, the slider, Play, a check button, opening the pick or the save dialog cancelled.

## 13. Empty, error, mixed and large states

- **Nothing runs:** section 6.3; Export disabled; the Toolpath view draws only the fixtures and grid, the bar shows "0 s of 0 s · 0 mm of 0 mm" with Play and the slider disabled (P20).
- **Laser (off), other page kinds:** no Output tab, no Route toggle.
- **Generic laser:** the warn; Machine line 1 names it.
- **Damaged profile:** block (c); Machine lines read "Machine profile damaged".
- **Large:** 50 runs: the Runs list is windowed above 20 rows; 1,000 items: arrowheads skipped under 12 px, at most 2,000 per frame, rings always; the budgets of criterion 36 (pan, zoom and slider within 16 ms, the job built lazily, only cut at a length for the slider).

## 14. Decisions for veto (Proposal)

- **P1** Route toggle after the Operations view toggle, same look, unpressed by default.
- **P2** The two page-label toggles are exclusive; releasing Route restores ListTree.
- **P3** Rings and run rows share one number; the fixture jobs are run 1 (one per kind they run).
- **P4** Engrave scan lines thinned to 3 to 6 screen px; one arrowhead per Engrave area.
- **P5** Travel inside one Engrave area is not drawn (counted).
- **P6** No arrowhead on a path shorter than 12 screen px; at most 2,000 per frame.
- **P7** After the head: 35 % opacity, 60 % width; head = ink ring with casing and dot.
- **P8** Play bar 10 px under the page frame, sticky at the canvas bottom, only while the Toolpath view is pressed.
- **P9** The slider's axis is the job's time.
- **P10** The slider starts at the end; Page Up and Down step 10 %.
- **P11** The readout wraps under the slider below 480 px.
- **P12** An edit while playing pauses; reduced motion does not stop Play.
- **P13** Tab order Machine, Start, Runs, Checks, Total, Export; each run's time in its row (the summary is not a second list).
- **P14** Block glyph in `--destructive`, warn glyph in ink.
- **P15** Check buttons: items select on the canvas, an operation opens its unfolded row in the Operations tab, fixture jobs focus the pick.
- **P16** Export as a full-width Primary with symbol and words, `aria-disabled` so the block stays reachable.
- **P17** The fixture pick shows only while a fixture job has Output on.
- **P18** Machine line 2 ends with " · air assist" when on.
- **P19** One export notice with a second line for the fixture jobs, 3 s or 5 s.
- **P20** Nothing runs: the bar's controls are disabled, not hidden (the bar keeps its place).
- **P21** Air assist is a switch row after Motorised Z axis, one undo step.
- **P22** The Head position warn reads "Starts where the head stands; the bed is not checked." (the spec's text is 74 characters).

## 15. Mismatches for the product owner and design-system changes

- **M1** Criterion 10 lists "one line per operation and a total" in a summary below the gate; the notes put each run's time in its gate row and keep only the total as the summary (P13).
- **M2** Criterion 37's test ("slider at half the total length reads half the total time") holds only for one feed; with P9 the test becomes "slider at half reads half the total time, and the drawn lengths up to the head equal the file's up to that move".
- **M3** Criterion 38's warn text has 74 characters (P22).
- **M4** Criterion 2 writes each operation as one line; in 244 px it takes three lines (6.2). The fixture jobs line gets a run number (P3).
- **M5** Export (criterion 4) and Start from (criterion 38) are disabled, not hidden, as the spec asks; both keep their place and name the reason in a tooltip. Design-system exceptions to "Hidden, not disabled".
- **M6** Criterion 2 does not state Air assist in the gate (P18).
- **M7** Criterion 27 has two notices for one export; one notice with two lines (P19).
- Design-system rows to add at acceptance: "Toolpath view toggle (`0029`)" in "Kind chip places", "Toolpath look (`0029`)", "Play bar (`0029`)" (the one slider of the app), "Output tab (`0029`)", "Check line (`0029`)", "Air assist row" in the Machine block. No token.

## 16. Implementer checklist

- Page label: Route toggle after ListTree, exclusive pair, session state.
- Toolpath drawing from the export's move builder: kind looks, thinning, travel, rings, arrowheads, pale after the head, head ring; redraw live.
- Play bar: placement and sticky rule, Play/Pause, time-axis slider with keys, readout and its wrap.
- Output tab: strip entry; Machine lines; Start from and Home first (disabled rule); fixture pick (visibility, unpickable rows); run rows (three lines, source, typed, time); checks (order, glyphs, buttons, sr prefixes); total; Export (aria-disabled, tooltip, description).
- Export flow: file name, dialog, notice texts, one step "Export G-code", error dialog, cancel without step.
- Machine block: Air assist switch row and its step.
- Texts of section 10 at 70 characters at most; no new colour, no popup, no operating tip; budgets of criterion 36.

## Links

Spec: `specs/0029-laser-job-preview-and-output/specification.md`. Drafts: `docs/design-drafts/2026-11-laser-output/` (01 to 03). Built notes: `specs/0101-laser-operations/ux-notes.md`, `specs/0111-laser-operation-tools/ux-notes.md`, `specs/0028-material-test-library/ux-notes.md`, `specs/0113-fixture-sets-and-pins/ux-notes.md`.
