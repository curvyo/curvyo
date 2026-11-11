# 0103 Fixture jobs: UX notes

ux-engineer, 2026-11-11. Checked against `specification.md` (criteria 1 to 26, Fragen 176 to 178 with their defaults A, B, B), `specs/0101-laser-operations/ux-notes.md` (the job rows, the armed trash, the rename, the look, the summary; 2026-11-10 rework), `specs/0102-laser-fixtures/ux-notes.md` (the fixtures, their kind symbols, the Fixtures block, the Layers Fixtures row; 2026-11-10 rework), `docs/design-system.md` ("Interaction conventions": "Inline delete confirm", "Armed action", "List row", "Canvas notice slot", "Hidden, not disabled"; the "All formats" disclosure row of `0045`; "Drop badge", "Drag ghost", "Casing over artwork"). Customer rules applied: no operating tips or hint labels (2026-10-16), no notice after a success (2026-10-31), hidden not disabled, no popups in the panel, no cut or engrave settings in a row (2026-11-10), symbols plus text, in-place rename by double-click, the armed trash, arrows and drag for order, no invented colours, few clear things. Items marked (Proposal) are vetoable at the demo; section 15 lists them as J1 to J16.

Terminology: "job" is a fixture job. "Part" is a fixture part (`0102`), "target" is a part ticked in a job (`0103` criterion 9). "Ink" is `--toolbar-icon`.

## 0. What is reused

| Part | Reused from | Change in this slice |
|---|---|---|
| Section header | the "All formats" disclosure row (`docs/design-system.md`, `0045`): 28px, chevron 12px, label 14px | label "Fixture jobs (N)" |
| Job row | `0101` section 4.2 (40px, glyph, name, Show, Output, armed trash, arrows, line 2) | no digit, no tick, no grip |
| Kind symbols | `0101` section 3 (the five work symbols) and `0102` section 4.0 (fixture symbols, group heading) | none new |
| Rename, order, armed trash | `0101` sections 4.4 to 4.6 | the group is the order range |
| Part ticks | the `.cb` check box of `0101` section 6.2 and of `0102` section 3.3 | ticks in the Parts list only |
| Look on the canvas | `0101` section 13 (line styles, hatch, chips, budget) | parts of a fixture, not items |
| Summary | `0101` section 7 | one line for fixture jobs |
| Refusal | `0101` section 6.3 (Drop badge "Not here") | a notice for work dropped on a job |

## 1. Principle

A fixture job belongs to its fixture. It is listed in the Operations tab under the work, grouped under the fixture it engraves or cuts, and it runs only the parts the maker ticked, and only while its Output is on. The look is the work's look: red only for lines that are cut, pale orange only for hatched areas that are engraved, ink for everything else. The section is collapsed by default, so the work list stays the main thing. Nothing opens a dialog or a popup, and no control appears that does not apply.

## 2. Tokens (no new colour)

| Use | Token | Contrast on the ground it sits on |
|---|---|---|
| Cut lines and Cut-out lines, Cut symbol | `--cut` (`0101`) | 4.81:1 on `--canvas-bg`; 4.30:1 on `--toolbar-bg` (board ground) |
| Engrave hatch, Engrave + Outline hatch | `--engrave-hatch` (`0101`) | 3.17:1 on `--canvas-bg`; **2.83:1 on `--toolbar-bg`**, so see J6 |
| Score line, every ink symbol, fixture ink | `--toolbar-icon` | 9.25:1 on `--canvas-bg`; 8.27:1 on `--toolbar-bg` |
| Output-off symbols, muted names, muted words | `--panel-muted-fg` | 5.61:1 on `--canvas-bg`; 5.01:1 on `--toolbar-bg` |
| Not assigned outline (Output off) | `--lane-line` (dashed 4 on, 3 off) | 3.45:1 on `--canvas-bg` |
| Casing under every line | `--selection-casing` | |
| Armed row and armed square | `--destructive-soft`, `--destructive` (`0081`) | as `0101` section 4.5 |
| Selected row, focus ring, field border | `--value-fill`, `--accent`, `--editor-accent` | as `0101` |

No value is added. The section header, the group headings and the target rows use the panel tokens of `0101`.

## 3. The Fixture jobs section (criteria 1 to 3, 17)

### 3.1 Place and header

- Operations tab, top to bottom: add group (`0101`), Cuts last, the work list (flexible, minimum 120px), **Fixture jobs section**, summary (`0101` section 7, with the fixture line of section 7 here), View switch. J1 puts the section between the list and the summary, since criterion 1 says "below the work list" and the summary counts both.
- Header: a button, 244 x 28px, full width. Chevron 12px `ChevronRight` when collapsed, `ChevronDown` when expanded. Label "Fixture jobs (N)", 14px `--toolbar-icon`, N the number of fixture jobs on the page, unknown ones included. `aria-expanded`, `aria-controls` the body. Hover `--editor-accent-hover`; focus 2px `--editor-accent` ring.
- Keys: Tab stop; Enter or Space toggles (a native button). No arrow keys.
- Collapsed by default. The expanded state is per viewer, kept for the session, not stored in the file, not an undo step.
- Collapsing or expanding changes the list, so it disarms an armed trash (`0081` list-change rule).

### 3.2 Body states

- **Expanded, no job:** one line "No fixture jobs. Select a fixture to add one." 14px `--panel-muted-fg`, not focusable, in the body's place. The add toolbar (3.3) stays above it.
- **Expanded, jobs:** add toolbar, the "Runs first" label (3.4), the fixture groups (3.4). Nothing else.
- **Body height:** no cap. The section grows with its rows and the tab scrolls as one column, so a collapsed header keeps the panel short (J2). The work list keeps its `0101` windowing and minimum height.
- **Collapsed:** the header only, with the count. Collapsed with zero jobs: "Fixture jobs (0)".

### 3.3 Add toolbar (criterion 2)

- `role="toolbar"` named "Add fixture job", at the top of the body, a 28px row. Five icon buttons, 28 x 28px (36 coarse), 4px apart, glyph only: the five work symbols of `0101` section 3 (Cut, Cut-out, Score, Engrave, Engrave + Outline). Left and Right move, Enter or Space presses; one Tab stop; focus stays on the pressed button, so a second press adds the next job.
- **Shown only while exactly one fixture is selected** (on the canvas, in the Fixtures block or in the Layers row). With no fixture, or several, the five buttons leave the tree (hidden, not disabled, J3). The 28px row stays, empty, so the list does not jump. No text. The criterion's "Select a fixture first." and its disabled buttons are not used (J3).
- Names and tooltips carry the target: "Add Cut job to Holder 1", "Add Score job to Board tray", and so on. Five names, all with the fixture's name.
- Press: one job of that kind is added under the selected fixture, named "<Kind> <N>" (N the lowest number not used by a fixture job of that kind on this page), Show on, Output off, selected, its Parts list open (5.1). One undo step "Add fixture job" (ADR 0052 §8). No notice (no success notice, 2026-10-31). The new job is the last one in its fixture's group. If the fixture has no group yet, the group appears.
- If the selected fixture goes away or the selection changes while focus is on a removed button, focus goes to the section header, never to `body`.

### 3.4 Groups and the "Runs first" label (criteria 3, 14)

- **"Runs first" label:** one row 24px, 11px semibold `--panel-muted-fg`, the words only, above the first group (the header style of `0101` section 4.1). It is the label of the run block under Frage 176 A. Under B the label reads "Runs after the work". Under C there is no block label; each job gets a choice (not built here, J13).
- **Group:** one `role="group"` per fixture that has at least one job, in the creation order of the Fixtures block (`0102`). A group with no job is not shown.
- **Group heading:** 28px row, the fixture's symbol 16px (`0102` section 4.0, `aria-hidden`), the fixture's name 13px semibold `--toolbar-icon`, ellipsis, tooltip with the whole name. Not a button, not a Tab stop. A press on the heading selects the fixture on the canvas (mouse route; the keyboard route is the Fixtures block row, `0102` section 10).
- Divider: 1px `--divider` above each group except the first.
- Inside a group the jobs run in their order (criterion 3). Between groups the order is the group order.
- Jobs of a fixture that is not Active still show in their group (4.2).

## 4. Job rows (criteria 4, 5, 15, 21)

### 4.1 Structure

One `listitem` per job, 40px high (48px on a coarse pointer), two lines like `0101` section 4.2.

**Line 1**, left to right:

| Slot | Width | Content |
|---|---|---|
| Glyph | 16px | the kind symbol, `aria-hidden` |
| Gap | 4px | |
| Name | flexible (about 80px, as `0101`) | 13px `--toolbar-icon`, one line, ellipsis; tooltip with the whole name; double-click or F2 renames |
| Show | 24px | `Eye` on, `EyeOff` off, `aria-pressed` |
| Output | 24px | `Power` on, `PowerOff` off, `aria-pressed`; new jobs start off |
| Trash | 24px | `Trash2` 16px, `--panel-muted-fg`; armed look in 4.5 |
| Group gap | 8px | |
| Up | 24px | `ChevronUp` 16px; hidden at the group's first row, slot kept |
| Down | 24px | `ChevronDown` 16px; hidden at the group's last row, slot kept |

**Line 2** (11px, `--panel-muted-fg`, full width): the kind word, " · ", then the first of these that applies:

1. the fixture is not Active: "not on the bed" (the word of `0102` section 3.3, J12). Example: "Cut · not on the bed".
2. Output is off: "Output off". Example: "Engrave + Outline · Output off".
3. the number of ticked parts: "1 part", "2 parts", "no part ticked".

Examples: "Engrave + Outline · 3 parts", "Cut · no part ticked", "Score · not on the bed".

- Row states: hover `--editor-accent-hover`; selected (its Parts list is open, 5.1): `--value-fill` ground and a 3px `--accent` bar at the left edge; focus 2px `--editor-accent` ring inside the row.
- Show off or Output off: the glyph shape changes (Eye to EyeOff, Power to PowerOff), the name goes to `--panel-muted-fg`, and line 2 says it. Muted ink plus a word, never opacity alone. A new job is muted at once, which is the point of Output starting off.
- Press on the glyph, name or line 2 selects the row. Enter or Space on the row selects it. A selection of a row does not change the canvas selection (as `0101` decision 6).
- No digit route: a job is not assigned from the canvas, so line 2 shows no number.
- No margin and no settings field: a Cut-out job runs with margin 0 (ADR 0052 §1, `0103` criterion 19). The row type has no settings.

### 4.2 Accessible names (row)

"<name>, <kind>, <n> parts" with ", Output off" or ", not on the bed" where they apply, e.g. "Cut 1, Cut, 0 parts, Output off". `aria-posinset` and `aria-setsize` count over the whole section, groups included.

### 4.3 Unknown job (criterion 21)

- Glyph `CircleHelp` 16px `--panel-muted-fg`, name kept, line 2 "Unknown fixture job".
- Only the trash and the rename. No Show, no Output, no arrows, no Parts list (the criterion says "can only be renamed or deleted", J10). A press selects nothing and opens nothing.
- It keeps its place in its group. Other rows' arrows move past it.
- Accessible name: "<name>, unknown fixture job".

### 4.4 Show and Output (criteria 5, 13, 16)

- **Show** (view only): off draws the job's parts in fixture ink, without the kind look (section 6, criterion 19). It never changes what runs.
- **Output** (what runs): off means the job runs nothing. A new job starts off (criterion 5). The switch is stored with the job.
- Each press is one undo step, named "Show on" / "Show off" or "Output on" / "Output off" (section 10, J9).
- No notice on either. The canvas changes at once; the summary changes in the same frame.
- Pointer targets 24 x 24px, 4px apart. Output and Trash are neighbours; an unintended press on the trash only arms it (4.5).
- **Nothing in this slice turns Output off by itself.** Frage 178 B and criterion 6 move to `0029` (the page output, ADR 0052 §5 and `0103` `adrs.md` §5). Its step and its notice are `0029`'s; a note for that spec is in section 11.

### 4.5 Rename and delete (criteria 4, 3)

- **Rename:** double-click on the name, or F2 on the selected row. The line 1 becomes a 13px text field (white ground, 1px `--editor-accent` border, 4px radius), from the glyph's place to the right edge. Show, Output, Trash, Up and Down leave the tree while it is open and return on commit or cancel. Enter commits; leaving the field commits; Escape cancels. 128 characters. Empty reverts. Names need not be unique. Field name "Name of <name>". Undo "Rename fixture job" (fixed name, ADR 0052 §8). Touch: double-tap.
- **Delete: the armed trash** of `0101` section 4.5 and `0081`, unchanged: idle 24 x 24 `Trash2`. First press arms (the row takes `--destructive-soft` on both lines; the trash becomes a 22 x 22 `--destructive` square with a white glyph). A second press at least 400 ms later removes the job with its Parts ticks. A press inside 400 ms is ignored. Disarm without notice after 4 s, on Escape, on a press elsewhere, when focus leaves the row, when the list changes. One armed trash in the app at a time.
- Armed names: "Delete Engrave 1" idle, "Press again to delete Engrave 1" armed. The status region says the same, "Press again to delete Engrave 1." (no sentence about parts: the ticks are not visible work). On removal: "Engrave 1 deleted." in the status region only. Undo "Delete fixture job".
- Keys: `Delete` on the focused row arms; `Delete` or `Enter` on the armed row removes; Left and Right reach the trash inside the row.
- Armed-trash conflict with the design system's "reversible action never asks" rule is the same as `0101` decision 8 and `0102` D9. It is followed as the customer asked (J11).

### 4.6 Order: arrows, drag and Alt-keys (criterion 3)

- **Arrows:** Up and Down buttons, as in 4.1. Each press moves the job one place inside its group and is one undo step "Move fixture job". Focus follows the job (same arrow, else the other arrow, else the name, never `body`). The job stays selected.
- **A job cannot leave its group.** The arrows stop at the group's ends; the drag refuses to leave the group.
- **Drag:** press the glyph or the name and move. Starts after 4px (a click never reorders). A 2px `--accent` insertion line inside the group only. The source row stays in place with a 1.5px dashed `--drag-ghost` outline; a ghost row at 60 % follows the pointer vertically, clamped to the section (the `0100` Drag ghost rule). A press on a button never starts a drag.
- A drop outside the group: the ghost snaps back, `cursor: not-allowed`, no text, no step (as `0101` criterion 5).
- Escape during the drag cancels, no step. Release: one undo step "Move fixture job".
- **Alt+Up, Alt+Down** move one place in the group; **Alt+Home, Alt+End** to the group's first and last place. At the group's end: refused, no step, no text.

## 5. Parts and ticks (criteria 8 to 12)

### 5.1 When the Parts list shows

- Under the selected job's row, directly below it, a `role="group"` named "Parts of <job name>". Only one job is selected at a time, and a newly added job is selected.
- Layout: a 24px header "Parts" 11px semibold `--panel-muted-fg` at the left, and the count "2 of 5" (ticked of the readable parts) 11px tabular `--panel-muted-fg` at the right. Then one row per part, indented 16px (under the glyph's slot), 28px high (40px coarse).
- Row: `.cb` check box 16px (`role="checkbox"`, `aria-checked`), gap 8px, part name 13px `--toolbar-icon`, ellipsis, and at the right a text button "Select" (12px, 24px high, underlined on hover and focus).
- A deselected or unknown job shows no Parts list.
- The Parts list is rendered in full: one job has at most 51 parts (the board and 50 slots, `0102`), so it needs no window. It grows with the section (J2).

### 5.2 The parts of each kind (criterion 8, in this order)

| Fixture kind | Part rows | "Select" selects |
|---|---|---|
| Grid | "Grid lines", "Tapes" | the grid (no part selection exists, J5) |
| Holder | "Board", then one row per readable slot in creation order, named as the slot | the holder (Board) or the slot |
| Clamp | "Clamp outline" | the clamp |
| Stop | "Stop" (both legs as one part) | the stop |
| Screw | "Head", "Thread" | the screw |

- A slot that cannot be read (criterion 12) is not a row. Its ticks stay in the file, unchanged, and the count "of" leaves it out. The damage notice of `0102` covers it.
- A slot that is deleted drops its ticks in the same step, silently (criterion 11). Undo restores them.
- A moved, resized or renamed slot keeps its ticks (criterion 11); its row is renamed in place.

### 5.3 Ticks and Select (criteria 9, 10)

- A tick press is one undo step: "Assign to fixture job" on an unticked part, "Remove from fixture job" on a ticked one (the fixed names of ADR 0052 §8; the undo name carries no job name). Tooltip "Assign to <job name>" or "Remove from <job name>". Accessible name "<part name>", description "Part of <job name>". The stored key is `target.<node id>` (ADR 0052 §2); the UI word stays "Parts".
- Ticks are made in this list only. Nothing on the canvas and nothing in the Fixtures block ticks a part (J4). A press on a fixture on the canvas selects the fixture, not a part.
- "Select" selects the part on the canvas, as in 5.2. Undo: none (a selection is not a step). The Operations tab keeps its job selected.
- Work items cannot be dropped on a job (criterion 10). A drop of a work item on a job row is refused, see section 11.

### 5.4 Keys inside the Parts list

- Tab order: the add toolbar, the job list (one stop), the Parts list of the selected job (one stop, roving), then the summary.
- Up and Down move between part rows; Home and End to the first and last row; Page Up and Page Down by 5 rows; no wrap.
- Left and Right move between the check box and the Select button of the row.
- Space or Enter presses the focused check box or Select button.
- Escape in the Parts list returns focus to the job row and keeps the job selected. A second Escape does what it does on the canvas.
- Up from the first part row returns focus to the job row (not a job move).

## 6. The look of a job's parts on the canvas (criteria 18, 19)

### 6.1 Which look a part gets

Checked in this order; the first that applies decides:

| State of the part | Look |
|---|---|
| Show off on its job, or the job's kind is unknown | fixture ink, as `0102` section 5 (no kind look, no chip) |
| Ticked in a job with Show on and Output off | Not assigned look: 1px dashed `--lane-line` outline (4 on, 3 off), no hatch, no line of the kind; its symbol in `--panel-muted-fg` (J5) |
| Ticked in a job with Show on and Output on (fixture Active) | the kind look of 6.2 |
| Not ticked in any shown job | fixture ink, as `0102` (never dimmed by the job; J5) |

- A part ticked in two jobs takes the look of each job in list order, the later one on top (`0101` section 13.2, "several operations on one leaf").
- View: Drawn (`0101` section 2): no kind look, no chips, fixture ink, as `0101`.
- A fixture that is not Active is not drawn (`0102` 6.5), so no look.

### 6.2 The kind look on parts

Same lines and hatches as the work (`0101` section 13.2), on each part's own outline or area:

| Kind | On the outline of a part | On the closed area of a part |
|---|---|---|
| Cut | solid `--cut` line, 1 device px, on each outline | (none) |
| Cut-out | dashed `--cut` 2px, dash 6 on 3, on the outer contour of the part only | (none) |
| Score | dotted `--toolbar-icon` 1.5px, round dots every 3px, on each outline | (none) |
| Engrave | (none) | `--engrave-hatch` 1.5px at 45 degrees, 6 device px apart, screen space, clipped to the area |
| Engrave + Outline | 1px solid `--toolbar-icon` outline | the same hatch |

- Area of a part: the board (its whole rectangle, under the slots: the slot holes keep the bed as in `0102`); a slot (its hole); the clamp (its rectangle); the stop (the L as a whole); the screw's head (its disc) and thread (its disc); the grid's tapes (the band); "Grid lines" has no closed area.
- **Board ground under an engraved area:** the board's `--toolbar-bg` ground gives the hatch 2.83:1, below the 3:1 of criterion 18. Where a board area is engraved, the ground under the hatch is `--canvas-bg` (3.17:1), so the bed shows through, the way an engraved board looks (J6).
- Clamp: it has no ground of its own, so its hatch sits on the canvas (3.17:1) and needs no change. Its own ink hatch (`0102` 5.3) is replaced by the kind look while it is ticked with Engrave.
- **Grid lines with Engrave or Engrave + Outline:** no closed area, so only the symbol shows (and the ink outline for Engrave + Outline, which is the lines themselves). Proposal J14.
- Casing: every kind line has the 1px `--selection-casing` on each side (`0101` 13.2).

### 6.3 Symbols on the parts (View: Operations only)

- One 22 x 22px chip per kind, per part, as `0101` section 13.3: `--toolbar-bg` ground, 1px `--lane-line` border, 4px radius, the 16px symbol at 2px inset, the anchor 10px right of the part's bounding box top-right corner and 6px above its top edge, mirrored to the top-left corner at the viewport edge. Grid lines anchor to the work area's top-right corner.
- Glyph colour: Cut in `--cut`; every other kind in `--toolbar-icon`; a kind whose only jobs have Output off in `--panel-muted-fg`. If one kind has jobs with Output on and off, the Output-on colour wins.
- A chip never covers a handle (`0101` 13.3 rule) and never sits on the selected part's handles; pointer-events none; collisions skipped (`0101` 13.4).
- Chips of parts whose job has Show off: none (as 6.1).

### 6.4 Budget

- The chips share the 500-per-frame cap and the collision skip of `0101` section 13.4. Order: selected items first, then the parts of running jobs, then the rest in z order. The summary carries the count of every item and part skipped.
- The look itself is one clipped pattern fill per area, no per-line geometry (as `0101`).

## 7. The summary (criterion 17)

- Lines, in this order, 12px `--toolbar-icon`, tabular:
  1. "Runs <N> operations, <M> items.", only when N is at least 1 (J7). Plural and singular as `0101`.
  2. "1 fixture job runs." or "<F> fixture jobs run.", only when F is at least 1. F counts jobs with Output on, Show either, a known kind, a ticked part and an Active fixture.
  3. The `0101` lines "<K> items not assigned.", "<J> items run while hidden.", "<H> items sit in operations with Output off.", each only above zero.
- "Nothing runs. Assign items to an operation." only when neither a work operation nor a fixture job runs.
- The count of line 2 is a text button: it selects the fixtures of those jobs on the canvas and in the Layers tree (the multi-selection of `0101` section 7 for fixtures). Underlined on hover and focus, no icon, 24px row.
- Live region `role="status"`, `aria-atomic`. The summary updates in the same frame as any change.

## 8. The Layers tree (criterion 12; amends `0102` section 8)

- **Fixture child row** (`0102` 8): gains a chevron 12px at its left only when the fixture has at least one job. `aria-expanded`. Collapsed by default (J15). The row stays a press-to-select row; the chevron has its own press.
- **Job rows** under an expanded fixture: 28px, `treegrid` row at `aria-level` 3, the glyph aligned under the fixture's glyph (indent 2 levels), the kind symbol 16px, name 13px, and at the right the word "Output off" 11px `--panel-muted-fg` when Output is off. Unknown jobs: `CircleHelp`, name, "unknown". No eye, no lock, no rename, no drag, no Delete.
- A press or Enter on a job row selects its fixture on the canvas. It does not open or change the job in the Operations tab.
- The Layers group eye of the Fixtures row (`0102` D14) hides the fixtures; their jobs follow the fixture's state and have no eye of their own.
- Accessible name of a job row: "<name>, <kind>, fixture job, Output off" (or without the words where they do not apply).

## 9. Profile, pick and damage (criteria 22 to 24)

- **No new control.** "Fixtures in profile: N" keeps counting fixtures (J16 keeps the count unchanged). "Save as profile", "Update profile" and "Reset from profile" (`0102` 9.1) write or copy the jobs with the fixtures; "Update profile" counts a job change as "Differs from profile".
- **Pick profile:** every copied job starts with Output off, with fresh ids for the parts. A part token with no counterpart on the copied fixture damages the profile record, as ADR 0052 §7 says; nothing is dropped and there is no drop notice here. The jobs appear in the same undo step as the fixtures.
- **Damaged values** (criterion 22): one notice "1 value damaged on Engrave 1 job." (or "<n> values damaged on <name>."), `0102` pattern, status, 5s. Show reads on, Output reads off, the file is not rewritten on read.

## 10. Undo steps (one per gesture)

"Add fixture job", "Rename fixture job", "Delete fixture job", "Show on" / "Show off", "Output on" / "Output off", "Move fixture job" (drag, arrows and Alt-keys), "Assign to fixture job", "Remove from fixture job". These are the fixed names of ADR 0052 §8 (J9): no job name, no part name in an undo name. The output step of `0029` carries its own label. "Remove fixture" removes its jobs and their ticks in the same step (criterion 7). "Reset fixtures from profile" as `0102`. No step for a selection, a row selection, the section's expanded state, a Parts list opening, View or a focus move. Profile library writes are not in undo (`0026`).

## 11. Notices, status and refusals

| Text | Where | Role and time |
|---|---|---|
| "Work items do not run in fixture jobs." | canvas notice slot; a drop of a work item on a job row, the row takes nothing | `role="alert"`, 8s (refusal look); the drop badge is `Ban` "Not here" while over the row (`0101` 6.3) |
| Fixture dropped on a job row | Drop badge `Ban` "Not here", `not-allowed`; no notice, no step (J8) | |
| "1 value damaged on Engrave 1 job." | canvas notice slot, on opening the file | status, 5s |
| "Press again to delete Engrave 1." / "Engrave 1 deleted." / "Engrave 1 added." | the section's sr-only polite status region | polite; nothing visible on removal or add |
| "Press again to delete Holder 1. Its 2 slots and 1 fixture job go with it." | status region of the Fixtures block, on arming (amends `0102` 3.5, which named only slots) | polite, sr-only |

Not used: any success notice for add, rename, move, tick, Show or Output (2026-10-31); any hint line about parts or the order (2026-10-16); "Select a fixture first." (J3); "Fixture inactive" (replaced by "not on the bed", J12).

Note for `0029` (not built here, J8): the page output switches off every job whose Output is on, in one step with one undo. Its status notice "Output turned off on N fixture jobs." (canvas notice slot, status, 5s) and its undo name belong to that spec (Frage 178 B, criterion 6, ADR 0052 §5).

## 12. Keyboard routes

| Action | Route | Notes |
|---|---|---|
| Reach the section | Tab in the Operations tab order: the work list, then the section header | the header is one Tab stop |
| Expand or collapse | Enter or Space on the header | section 3.1 |
| Add a job | Tab to the add toolbar, Left and Right, Enter or Space | needs one selected fixture; focus stays |
| Move between job rows | Up and Down, Home and End, Page Up and Page Down (5 rows); no wrap | roving focus, one Tab stop for the list |
| Select a job | Enter or Space on the row | section 4.1 |
| Rename | F2 on the selected row; double-click is the mouse route | Enter commits, Escape cancels |
| Show, Output, Trash, Up, Down | Left and Right in line 1 order; Space or Enter presses | Up and Down keys still move between rows |
| Arm and delete | `Delete` on the focused row arms; `Delete` or `Enter` on the armed row removes | Escape disarms |
| Reorder | Alt+Up, Alt+Down (in the group); Alt+Home, Alt+End | refused at the group's ends, no step |
| Enter the Parts list | Tab from the job list when a job is selected | one Tab stop, roving |
| Tick a part | Space or Enter on the check box | section 5.4 |
| Select a part | Space or Enter on its Select button | |
| Leave the Parts list | Escape returns to the job row; Tab goes to the summary | |
| Summary count | Tab to the count button, Space or Enter selects those fixtures | section 7 |
| Undo | Ctrl+Z, as built | every step of section 10 |

Notes: no letter key is bound to a kind, a job or a part (the letters are tool letters). Every drag has a keyboard route (Alt-keys, the arrow buttons and the Parts ticks), so no drag is needed (WCAG 2.2 dragging alternative). Digits do not assign anything in this section (no digit route, 4.1).

## 13. Accessible names (complete)

| Element | Name | State or description |
|---|---|---|
| Section header | "Fixture jobs (N)" (button) | `aria-expanded`, `aria-controls` |
| Section body | region named "Fixture jobs" | |
| Add toolbar | "Add fixture job" | |
| Add buttons | "Add Cut job to <fixture>", "Add Cut-out job to <fixture>", "Add Score job to <fixture>", "Add Engrave job to <fixture>", "Add Engrave + Outline job to <fixture>" | only with one fixture selected |
| "Runs first" label | "Runs first" (no role) | |
| Job list | "Fixture jobs" (list) | |
| Fixture group | "<fixture name>, <kind>" (group, labelled by its heading) | |
| Job row | "<name>, <kind>, <n> parts" plus ", Output off" or ", not on the bed" | `aria-posinset`, `aria-setsize` over the section, `aria-selected` on the selected row |
| Show | "Show <name>" | `aria-pressed` |
| Output | "Output <name>" | `aria-pressed` |
| Trash | "Delete <name>"; armed "Press again to delete <name>" | tooltip the same |
| Up, Down | "Move <name> up", "Move <name> down" | hidden at the group's ends |
| Rename field | "Name of <name>" | open only while renaming |
| Unknown job row | "<name>, unknown fixture job" | trash and rename only |
| Parts list | "Parts of <job name>" (group); header "Parts", count as description | |
| Part check box | "<part name>" (checkbox) | `aria-checked`; tooltip "Assign to <job name>" or "Remove from <job name>" |
| Select | "Select <part name>" | |
| Summary count | "1 fixture job runs." or "<F> fixture jobs run." (button, named by its line) | selects those fixtures |
| Layers chevron | "Show jobs of <fixture>" / "Hide jobs of <fixture>" | `aria-expanded` |
| Layers job row | "<name>, <kind>, fixture job, Output off" | `aria-level` 3 |
| Canvas look, chips | `aria-hidden` (the canvas is not in the accessibility tree) | the kinds are read from the rows and the summary |

Live regions: the section's polite status (3.2 and section 11), the summary (`role="status"`, `aria-atomic`), the canvas notice slot (`role="status"` or `role="alert"`).

## 14. Empty, error and large states

- **No fixture, no job:** the section header "Fixture jobs (0)", collapsed; expanded, the empty line of 3.2.
- **Fixture with no job:** no group for it; the add buttons appear when it is selected.
- **Job whose parts are all unticked:** "Cut · no part ticked"; it runs nothing; the summary does not count it.
- **Fixture removed:** its group and its jobs go in the same step (section 10); the section's count drops.
- **Only unknown jobs:** the groups list the unknown rows; the summary says "Nothing runs. Assign items to an operation." when no other job runs.
- **Large (criterion 26):** 50 jobs in the section: fixed 40px rows, the section grows with them and the tab scrolls as one column. Above 60 rows the job rows are windowed against that scroll (the focused and the selected row kept when they scroll out, the first rows painted at once, the scrollbar exact from the first frame). The Parts list of one job is at most 51 rows and is rendered in full. A press of one tick updates one row, the look of the parts and the summary; no full re-render of the list. The timing budgets of criterion 26 belong to the architect (`adrs.md`); the UX side adds nothing to them.

## 15. Decisions for veto (Proposal)

- **J1 Place.** The section sits between the work list and the summary (criterion 1 says "below the work list"). Alternative: below the summary and the View switch.
- **J2 No cap, one scroll.** The section has no height cap and no scroll of its own; it grows with its rows and the tab scrolls as one column, so a collapsed header keeps the panel short. Rejected by the coordinator on the first draft (a second inner scroll area).
- **J3 Hidden, not disabled.** Criterion 2 says the five buttons are disabled and "Select a fixture first." shows. The house rule (`docs/design-system.md`, "Hidden, not disabled") says the buttons leave the tree and no sentence explains why. The 28px row stays empty so the list does not move. Decide the criterion or the rule.
- **J4 Ticks only in the Parts list.** No tick in the Fixtures block rows, none on the canvas (criterion 9). The lead's option "ticks in the Fixtures block rows" is not taken, so a fixture's rows stay as `0102` made them.
- **J5 Parts of an Output-off job.** Criterion 18 says a fixture part is never dimmed as Not assigned. This note draws a ticked part of an Output-off job in the Not assigned look (dashed `--lane-line`, muted symbol), as the lead asked, because "Output off" must show on the canvas the way it does for the work (`0101` decision 4). Criterion 18 must change. Also "Select" on grid parts selects the grid, since no part selection exists.
- **J6 Engraved board ground.** The hatch on the board ground `--toolbar-bg` measures 2.83:1, under the 3:1 criterion 18 asks for. Where a board area is engraved, its ground under the hatch is `--canvas-bg` (3.17:1). This is the one place a fixture part changes its ground. Alternative: a white casing 1px inside the hatch (busier).
- **J7 Summary lines.** The work line "Runs <N> operations, <M> items." is omitted when no work operation runs, so the fixture line is the first line. "Nothing runs." only when neither runs.
- **J8 Refusals; output notice moves to `0029`.** Work dropped on a job row gets one `role="alert"` notice (8s). A fixture dropped on a job row is refused silently with the Ban badge. The Frage 178 B turn-off and its notice are not built in this slice: nothing outputs a page yet (ADR 0052 §5). The note for `0029` is in section 11.
- **J9 Labels.** Undo names are the fixed names of ADR 0052 §8 and `0103` `adrs.md` §7 ("Assign to fixture job", "Show on", "Output off"), not "Hide Cut 1" or "Assign to <job name>". Tooltips still name the job.
- **J10 Unknown job.** No arrows, no Show, no Output, no Parts list, as criterion 21 says ("can only be renamed or deleted").
- **J11 Armed trash.** Jobs use the armed trash (`0081`), as operations (`0101` decision 8) and fixtures (`0102` D9). The design system's rule "a reversible action never asks" is the one to weigh; decide all three together.
- **J12 Words.** Line 2 says "not on the bed" for an inactive fixture (the `0102` word), not "Fixture inactive" (criterion 15). Priority: not on the bed, then Output off, then the part count.
- **J13 Frage 176 C.** Not built here. If chosen, each job gets a two-state text choice "Before the work" / "After the work" in its Parts block; the "Runs first" label goes.
- **J14 Grid lines with Engrave.** Open lines have no closed area, so only the symbol shows; Engrave + Outline shows the grid lines in ink, as they already are.
- **J15 Layers.** A fixture with jobs gets a chevron, collapsed by default; job rows have no eye and press to select the fixture.
- **J16 Profile count.** "Fixtures in profile: N" keeps counting fixtures only; the job count is not added to the row.

## 16. Changes asked of the product owner, and the design-system rows

Criteria to change in `specification.md` (the PO owns the spec; the UX notes do not edit it beyond the pointer):

- Criterion 2: disabled buttons and "Select a fixture first." become hidden buttons and no text (J3); the names carry the fixture (3.3).
- Criterion 4: the undo names are the fixed names of section 10 (J9).
- Criterion 6 moves to `0029` with its notice (J8, ADR 0052 §5): the page output switches off the jobs, and the note of section 11 goes with it.
- Criterion 15: "Fixture inactive" becomes "not on the bed" (J12).
- Criterion 17: the work line is omitted when no work runs (J7).
- Criterion 18: "never dimmed as Not assigned" is replaced by the look of 6.1 (J5); the engraved board ground is in J6.
- Criterion 9 and the Parts list: the "Select" button selects the part or its fixture (5.3, J5).
- `0102` section 3.5 and section 8, and `0101` section 1 (the placement, J1) get the amendments named above. No scroll amendment (J2 rejected).

Design system rows to add at acceptance (not added now, as `0101` and `0102` did for their rows): "Fixture jobs section (`0103`)" (header, body, toolbar, groups), "Fixture job row (`0103`)" (line 2 words, the Parts list), "Fixture part look (`0103`)" (section 6). No token is added. J6 is recorded in the row, not as a token.

## 17. Review checklist for the implementer

- Section: collapsed by default; header "Fixture jobs (N)" with `aria-expanded`; no height cap, grows with its rows; the empty line; the add toolbar hidden unless one fixture is selected, names carry the fixture.
- Groups: one per fixture with jobs, creation order, heading with symbol and name, "Runs first" label, dividers.
- Rows: 40px (48 coarse); glyph, name, Show, Output, Trash, Up, Down; line 2 without a digit: kind, then "not on the bed", "Output off" or the part count; new jobs Output off, muted.
- Trash: `0101` armed look, timings, one armed at a time, status text, no notice on removal.
- Arrows: hidden at group ends, slot kept; drag starts after 4px; insertion line inside the group; refused drop snaps back without text; Escape cancels; one step.
- Parts list: under the selected job; "Parts" and "2 of 5"; ticks with undo labels; Select selects the part or its fixture; keys as 5.4.
- Canvas: part look per 6.1 and 6.2; Engrave hatch on a board with the `--canvas-bg` ground (J6); Output off = Not assigned look; Show off = fixture ink; chips per 6.3 under the cap of 500.
- Summary: work line only when it runs; "1 fixture job runs." with a button that selects the fixtures; "Nothing runs." only when none runs.
- Layers: chevron on fixtures with jobs; job rows at level 3; press selects the fixture.
- Names and keys as sections 12 and 13; no tips, no hint lines, no success notices.

## Links

Spec: `specs/0103-fixture-jobs/specification.md` (criteria 1 to 26, Fragen 176 to 178). Model: `specs/0101-laser-operations/ux-notes.md` (sections 3, 4, 5, 13), `specs/0102-laser-fixtures/ux-notes.md` (sections 3.3, 4.0, 5, 8, 9.1). Armed trash: `specs/0081-inline-delete-confirm/ux-notes.md`. Layers rows: `specs/0050-layers-and-objects-panel/`, `specs/0106-move-versus-edit/ux-notes.md`. Output step of the page: `specs/0029` (not yet written). Design system: `docs/design-system.md` ("Interaction conventions", "All formats" row of `0045`, "Drop badge", "Drag ghost", "Casing over artwork"). Concept: `specs/0063-manufacturing-workbenches/concept-output-pages.md` section 6.
