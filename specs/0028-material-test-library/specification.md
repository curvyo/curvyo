# Material test library: a power/speed test grid for the page's machine, the cell that worked, a reusable material record, settings on the operation

Status: Done (PR #333)
Priority: Must
Origin: Customer (R-MAT-001 to R-MAT-003; MVP slice 15; `0063` slice 6 "Settings and output"). Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

**Lessons of the reviews of PR #305, #313 and #330 (customer, 2026-11-10) applied here:** no invented colours (symbols and text; the ink, the red of the cut lines and the pale orange of the hatch only); in-place rename by double-click; delete as an armed trash button; order by arrows plus drag and drop; an object that is generated is edited like a rectangle (fields in the Properties panel, handles on the canvas); no clutter.

## User value

As a maker with a new sheet of material I want Curvyo to lay out a test grid of power and speed for my machine, to mark the cell that cut or engraved well, and to keep that as a named material, so that I never guess power and speed again and every later operation on that material takes the proven numbers with one pick.

**What other tools do.** LightBurn has a Material Test generator (a grid of squares with power and speed ranges, one layer, labels engraved) and a material library of presets per layer; the result of a test is not recorded, the maker types the numbers. xTool Studio and Glowforge ship material presets for their own machines only. Fusion 360 and Inkscape have nothing. We do better by (1) recording which cell worked, with the test kept as the source of the record, (2) tying the record to the machine, and (3) keeping the grid an editable object on the page.

**One-line meaning (Proposal, to confirm at the demo):** a *material record* is a name, a thickness and, for one machine, the power, speed and passes that were proven by a test cell for Cut, Score and Engrave. It is not a job and not a drawing object.

## Words (shown in the UI)

- **Test grid:** a generated block on the laser page. Columns are power values, rows are speed values, each cell is a square of the chosen kind, with the values engraved beside it. One kind per grid: **Cut**, **Score** or **Engrave**.
- **Cell:** one square of the grid with its own power and speed (and one pass).
- **Worked:** the one cell the maker marks as "this one is right". It is shown by a tick symbol and the word "Worked", never by a colour.
- **Material:** the record of the Words above. In the UI: "Material".
- **Settings:** power (percent of the machine's maximum power), speed (mm/min) and passes (whole number). A set of settings belongs to one kind: Cut, Score or Engrave.

## Where settings live (the decision of this slice)

`0101` and `0103` have no machine settings on purpose. This slice adds them, in this place only (Proposal):

1. Given a laser page, then the Operations tab (`0101`) has one more button, "Test grid", beside the five kind buttons. Pressing it adds one test grid at once with the remembered values of this machine or the defaults, no make block (rectangle model). The grid's fields (Kind: Cut, Score or Engrave, default Cut, a field; Columns (power) 2 to 12, default 5; Rows (speed) 2 to 12, default 5; Power from and to, percent of the machine's maximum power, 1 to 100 with at most one decimal, default 20 to 100; Speed from and to, mm/min as whole numbers within 1 and the machine's maximum feed, default 10 % to 100 % of it; Cell size 3 to 50 mm, default 10; Gap 0 to 20 mm, default 3) are edited in place in the Properties panel (criteria 8, 9). Grid values round to 0.1 % power and whole mm/min.
2. Given the grid fields, when a value is outside its range or the grid would not fit the bed of the machine, then the value is kept and marked, nothing is clamped, the grid stays as it was and the text names the cause ("Grid is 260 mm wide, the bed is 200 mm."). When the add button cannot place the grid (no free spot on the bed), the refusal is shown under the button. The values of the last grid are remembered per machine, **per computer** (in the config directory, not synced, not in the project; Proposal).
3. Given the add button, when the maker presses it, then one **test grid** is added on the active laser page, with the top-left corner at the top-left of the bed, or, if that spot is taken, the first free spot found left to right, top to bottom (Proposal); one undo step "Add test grid". It is selected. Nothing else on the page changes.
4. **Fixture jobs** keep having no settings (`0103`, customer 2026-11-10); `0029` decides what a fixture job runs with.
5. `0029` reads an operation's effective numbers by the rule of ADR 0054 §2: the live material's numbers (or the stored ones when the material is not found), with the operation's overrides applied over them. Its output commit refreshes the stored numbers beside the link, never the overrides.

## Acceptance criteria

"Laser page" means a page of kind Laser whose extension is on (`0098` criterion 18). The machine of the page is the profile copy of `0039`; if the page uses the built-in Generic laser, the records are tied to its id `generic-laser`.

### Making a test grid

1. Given a laser page, then the Operations tab (`0101`) has one more button, "Test grid", beside the five kind buttons. Pressing it adds a grid at once (criterion 3, UX M1) with the remembered values of this machine, else the defaults below; the grid's fields are then edited in place in the Properties panel (criteria 9, 10): Kind (Cut, Score, Engrave; default Cut), Columns (power, 2 to 12, default 5), Rows (speed, 2 to 12, default 5), Power from and to (percent of the machine's maximum power, 1 to 100, default 20 to 100), Speed from and to (mm/min, whole numbers within 1 and the machine's maximum feed, default 10 % to 100 % of the maximum feed), Cell size (mm, 3 to 50, default 10) and Gap (mm, 0 to 20, default 3). The fields name the machine ("for <profile name>") and show the size of the grid in mm.
2. Given the grid fields, when a value is outside its range or the grid would not fit the bed of the machine, then the field shows the allowed range, the value is kept and marked, the grid stays as it was and the text names the cause ("Grid is 260 mm wide, the bed is 200 mm."); when "Add test grid" cannot place a grid (no free spot on the bed) the refusal is shown under the button. Nothing is silently clamped. The values of the last grid are remembered per machine for the next grid (Proposal).
3. Given the button "Add test grid", when the maker presses it, then at once, with the remembered or default values, one **test grid** is added on the active laser page, with the top-left corner at the top-left of the bed, or, if that spot is taken, the first free spot found left to right, top to bottom (Proposal); one undo step "Add test grid". It is selected. Nothing else on the page changes.
4. Given the grid, then it consists of ordinary items on the laser page, no new object kind (Proposal): per cell one closed square (Cut, Score: its outline; Engrave: filled), one row of column labels above (the power, for example "40") and one column of row labels at the left (the speed, for example "3000"), each a text item in the document's default font, 3 mm high, plus one unit line "% power / mm/min" (Proposal). Squares are in the grid's own group, labels in a second group; both inside one top group named "Test grid <N>".
5. Given a grid of the kind Cut or Score, then cell squares are *not* closed by hatch and not engraved; given Engrave, then each cell is a filled square. The labels are always engraved (never cut). Given a Cut grid, then a cell is a closed square, not a point, so a cut-through cell drops out whole.
6. Given the grid, then its cells are assigned to operations so that each cell runs with its own power and speed: the list shows **one** row "Test grid <N>" (symbol of its kind, "<kind>, <C> x <R> cells") instead of one operation per cell (Frage 183, default A). The row has the same Show and Output switches, the in-place rename (double-click on the name, `0101` criterion 3), the armed trash (`0101` criterion 4) and the up and down arrows with drag and drop of `0101` criterion 5; it sits in the Marking section for Score and Engrave and in the Cutting section for Cut. Deleting the grid (its top group) deletes its row in the same step; deleting the row keeps the items, which then run nothing (they become Not assigned); items cannot be assigned to a grid row, and a grid row has no settings line. The grid's own items run with its row only: they are not assigned by hand to another operation (the notice "Test grid items run with their grid row only."), and a kind button pressed with a grid selected adds the operation empty (Proposal).
7. Given the labels, then they run with their own settings, set in the grid's fields as in LightBurn (Frage 186): a switch **Labels** (on by default), **Label kind** (Engrave, default, or Score; never Cut), **Label height** (mm, default 3), **Label power** (percent, default 20), **Label speed** (mm/min, default 50 % of the machine's maximum feed) and **Label passes** (default 1). With Labels off no label text is made. The labels always run before the cells in the same grid (so a cut grid does not drop its labels). This is the final field set; the unit line "% power / mm/min" is made with the labels.

### Editing the grid like a rectangle

8. Given a test grid, when the maker selects it (on the canvas or its row), then the Operations tab is shown and carries the grid's fields of criterion 1 (**Kind is a field**: changing it rebuilds the grid, clears the mark and moves the row to its section) and the label fields of criterion 7. There are no X and Y fields: the grid moves like a rectangle (drag, arrow keys, the Move chip). The canvas shows the handles of a rectangle selection (move and the bounding box; resizing a grid is not offered).
9. Given a selected grid, when the maker changes a field in the Properties panel, then the cells and labels are rebuilt in place in one undo step "Edit test grid", keep their group order, keep the *Worked* mark if the cell's power and speed are unchanged and clear it if not, and keep the Show and Output state of the grid row.
10. Given a grid whose items the maker has changed by hand (a cell moved, deleted or restyled, a label retyped), then the fields show "Edited by hand" and an enabled "Rebuild" button; nothing is rebuilt until the maker presses it (Proposal); Rebuild is one undo step and does not arm. The cells keep running with the settings they were made with, even when moved.
11. Given a copy of a grid (Ctrl copy, Duplicate, Paste), then the copy is a grid of its own with the same fields, no *Worked* mark, and its own row (Proposal).

### Recording which cell worked

12. Given a grid, when the maker selects it and opens the grid fields, then a result table is shown under them: the cells as a small table, power across, speed down, each cell a tick-box button. A cell is marked in the table only; clicking a square on the canvas highlights its table cell. The table and the canvas show the *Worked* cell with a tick symbol and the word "Worked" in the table's accessible name; no colour marks a cell.
13. Given a grid, when the maker marks a cell as Worked, then exactly that cell is marked and any other mark is removed; marking the marked cell again removes the mark. Each is one undo step "Mark cell" or "Unmark cell". The mark is stored with the grid in the project, so a project reopened later still shows it.
14. Given a marked cell, then the fields show its values in words, "Worked: 60 % power, 3000 mm/min", and the button "Save as material" (criterion 15). Without a mark the button is hidden, with no hint text.

### The material record

15. Given a marked cell, when the maker presses "Save as material", then an inline block asks for **Name** (text, 1 to 60 characters, default "Material <N>") and **Thickness** (mm, 0.1 to 100, one decimal, required), and a **Passes** field (1 to 20, default 1). Pressing the save button (and "Replace") is one undo step "Save as material" of the open project (Frage 189 B, criterion 32); undoing it takes the record or the set back and removes the history entry it added (Frage 190, default A). A pick "Add to" lists the existing materials of this machine plus "New material" (default). With "New material" a record is made; with an existing one the grid's kind set (Cut, Score or Engrave) is added to it, and an already present set of that kind is replaced only after the block shows "Replaces the <kind> set (60 %, 3000 mm/min, tested <date>)." and the maker presses "Replace" (the old set stays in the test history, criterion 18).
16. Given a saved record, then it holds (Proposal): a name; a thickness in mm; the machine it was tested on (profile id and name at the time); per kind a **set** (power percent, speed, passes) with its **source**: a reference to its entry in the record's test history, which holds the kind, the numbers, the grid and the date; no other value. A record is for **one machine**; for a second machine the maker tests again (R-MAT-001, "per machine/material combination"; the cross-machine comparison is R-MAT-004, out of scope).
17. Given the library, then records are kept in the maker's per-user data directory as one file per record, `materials/<uuid>.json`, with a `format_version` and a `modified` time, read and written as ADR 0004 §6 and ADR 0038 say for machine profiles (a damaged record is skipped and named, a record with a newer `format_version` is left untouched, reads never write); nothing leaves the machine (R-SYS-002, R-SYS-007). R-MAT-005 (sync across computers) is the existing folder mechanism of ADR 0038 and no new sync.
18. Given a record, then every test that was saved into it is kept as an append-only history inside the record's own record set (ADR 0004 §6: expensive to produce, never lost by an edit): the kind, the grid fields, the marked cell and the date. Replacing a set (criterion 15) or deleting it from the editor adds to the history and removes nothing from it; the one exception is Ctrl+Z of the step that added an entry, which removes exactly that entry (criterion 32, Frage 190 default A).

### Managing materials

19. Given the laser page, then the Operations tab has a collapsed section "Materials" (Proposal) listing the materials of this machine, in the maker's order (criterion 22: ordered materials first, new and unordered ones at the end, by name): per row the name, the thickness, and the symbols of the kinds it has sets for (a cut, score or engrave symbol each, the symbols of `0101`). Nothing is coloured.
20. Given a material row, when the maker double-clicks its name, then the label becomes an editable text in place (Enter or leaving commits, Escape cancels, empty reverts, 1 to 60 characters; names need not be unique) as `0101` criterion 3 says; there is no name field.
21. Given a material row, then it has an armed trash button as in `0101` criterion 4 (first press arms, a second press at least 400 ms later deletes, a press within 400 ms is ignored, disarms by itself, only one armed trash in the app); deleting removes the record file and its test history; it is one undo step "Delete material" of the open project, and Ctrl+Z in the same session (criterion 32) brings the record and its history back (Frage 189 B). Operations that point to it keep their stored numbers and show "Material not found, using stored numbers" (criterion 26). No dialog; the row's arming text names how many operations of open projects point to it when known (Proposal).
22. Given the materials list, then the maker changes its order with the up and down arrow buttons on the row and by drag and drop; Alt+Up and Alt+Down are extra keyboard routes (Proposal; the order is a user-library preference stored in a small order list, not in the records; materials not in the list come after the ordered ones, sorted by name). A material is edited like a rectangle, where it is an object: selecting a row shows thickness and the sets as fields (power, speed, passes per kind) in the panel, each editable in place; editing a set records the new numbers with the source "edited by hand" (the history keeps the old). A set edit or removal, a rename, a thickness change and a delete are each one undo step of the open project that also refreshes the stored numbers of every operation pointing to the material (Frage 189 B, criterion 32); the order is not a step. Materials not in the order list come after the ordered ones, sorted by name.
23. Given a material of another machine, then it is not shown in the list of this page (R-MAT-001); a "Show all machines" switch is not part of this slice.

### Using a material on an operation

24. Given a selected operation, then its settings are shown in a block below the list (kind symbol, "<power> %, <speed> mm/min, <passes> x"), every number an editable field (power 1 to 100, a typed power rounds to 0.1 %; speed 1 to the machine's maximum feed; passes 1 to 20). **Typing a number on an operation that points to a material keeps the link** (Frage 188 B) and is stored as an override of that number (one undo step "Edit settings"): the line shows "Differs from <name>" with the material's number beside each typed one, and a button "Reset to material" that removes the overrides (one undo step "Reset to material"); a number typed equal to the material's removes that override. The pick's "Own numbers" ends the link and keeps the numbers that ran. The line shows its source as one of three states (criterion 26): live, stored (with the reason) or own. An operation without either shows "No settings". The row of the list does not grow (the lesson "no machine settings in a step" stays true for fixture jobs).
25. Given a selected operation and a material of this machine, when the maker uses the **one material pick** in the settings block, then the operation stores the link and the numbers as last seen in one undo step "Use material"; a pick, also of the material it already points to, clears the overrides. The pick is the only route to another material, to no material and to own numbers; in the states live, stored and own the block shows the pick and "Edit material" (selects the material row, criterion 22), no further buttons besides "Reset to material" while overrides exist. The set used follows the operation's kind: Cut for Cut and Cut-out; Score for Score; Engrave for Engrave; for Engrave + Outline the Engrave set for the hatch and the Score set for the outline (the Engrave set if there is no Score set). A material without a set of the needed kind is listed in the pick as disabled with "No <kind> set". "No material" and "Own numbers" in the pick end the link and keep the numbers that ran as own numbers.
26. Given an operation that points to a material, then the block shows one of three states, each by a symbol and text, never by colour: (a) **live:** the material is found with the needed set; the block shows its current numbers and name; when the maker edits the material (criterion 22), every pointing operation changes in the same frame, except the numbers that carry an override. The stored numbers beside the link are refreshed in the same undo step as a material edit and on every save that changes the project, as well as at a pick and an output (Frage 187 B+C); no "stored" note is shown. (b) **stored:** the material is not found on this computer, its file is damaged or newer, it belongs to another machine, or it has no set of the needed kind: the line shows "Material not found, using stored numbers" (Proposal; for the no-set case "Material has no <kind> set, using stored numbers") with the stored numbers and name; (c) **own:** no link. In (b) the numbers are never changed, zeroed or guessed, the overrides are applied over the stored numbers; the file still runs and saves back unchanged, link and stored numbers kept. Opening the file where the material exists gives (a) with that computer's numbers; nothing in the project is written until a material edit, a save, a pick or an output.
27. Given a selection of several operation rows (Ctrl, Shift, Shift+arrows; amends `0101` criterion 3 and its UX note "one row at a time"), then the block shows only the material pick, and picking a material applies to all of them in one undo step and clears their overrides; an operation whose kind has no set in the material is skipped and named in a notice of at most 70 characters (Proposal).

### Files, failures, budgets

28. Given a laser page with settings on operations, grids with marks and material links with stored numbers, when it is saved and reopened, then all of it is as before, including an operation whose material is missing (criterion 26 (b)) (one golden project file in `tests/fixtures/`, `format_version` per the architect). A file of the previous format opens with no settings and no grids and saves back unchanged until the maker adds one. An operation of an unknown kind keeps its settings untouched (R-MFG-010).
29. Given a material file with an unknown key, a set with a power outside 1 to 100, a speed of 0 or above 100,000 mm/min (Proposal; ADR 0054 only requires at least 1), a thickness outside range, text over 60 characters, a set whose source names no history entry, more than 500 history entries, or a record text over 256 KiB, then the record is damaged: it is listed once as "Damaged material" in the Materials section with the reason, is not used and is never rewritten by a read; the other records load (the rule of ADR 0038). Golden files for each, `curvyo-laser-core/tests/fixtures/materials/`.
30. Given a grid of 12 x 12 cells, then making it takes within 50 ms, a field edit within 30 ms, and the Materials list with 200 records opens within 50 ms (thread CPU time, `curvyo-editor-wasm/tests/cpu_time/`; the architect confirms the limits).
31. Given the whole slice, then there is no network access, no device I/O, no G-code, no output file and no telemetry. The demo shows a grid that is ready to be cut and the numbers it taught; `0029` makes the output.
32. Given a material edit (Save as material, a set edit or removal, rename, thickness, delete; criteria 15, 21, 22), then Ctrl+Z and Redo reach it **only in the session in which it was made** (the session ends with New, Open or quit; Frage 189 B). If the record changed since the step (another edit, another computer's sync), Undo leaves the record as it is and shows a notice that names the material. A later session, another project and the actions of the History list restore only the project, never a record. Undo of "Save as material" or a set edit removes the history entry that step added; Redo adds it again (Frage 190, default A).

## Changes to accepted specs

| Spec | Criterion | Was | Now |
|---|---|---|---|
| `0101` | Out of scope ("Power, speed, passes and material per operation, the material library, the Score surface power (`0028`)") | out of scope | done in this slice (settings 24 to 27, undo 32); the Score surface power is the Score set of a material |
| `0101` | 1 (the Operations tab) | five kind buttons, Cuts last, the list, the summary | plus the Test grid button (1), the Test grid row (6), the settings line (24), the collapsed Materials section (19) |
| `0101` | 2 (a new operation) | an operation is added with a name, Show and Output | plus an empty settings set, "No settings" (Where settings live, 1); a material pick stores a link and the numbers as last seen (25) |
| `0101` | 20 (Score runs with the surface power of `0028`) | a reference only | the Score set of the operation (24, 25) |
| `0101` | 25 (save and reopen) | operations, memberships, Show, Output, Cuts last | plus the settings, the material link and stored numbers, the test grids and their mark (28); format bump |
| `0101` | 26 (unknown operation) | kept untouched | unchanged, settings included (28) |
| `0103` | Out of scope ("Cut and engrave settings per job ... (`0028`)") | out of scope | still out of scope; fixture jobs have no settings (Where settings live, 4) |
| `0026` | machine profile record | a profile is the machine description | unchanged; a material points to a profile id (16); no new key in the profile |
| ADR 0038 | §5 outlook ("When `0028` brings material records ...") | the record model sits in `curvyo-laser-core` | decided in the new ADR (below) |
| ADR 0004 | §6 (`materials/<uuid>.json`, test results append-only) | decided in principle | applied (17, 18) |
| `docs/requirements.md` | R-MAT-001 to R-MAT-003 | Must, unrefined | refined by 15 to 18; R-MAT-004 and R-MAT-005 unchanged |

## Out of scope

- Any output: G-code, a file, preview, time, the machine gate. The numbers are stored and shown; `0029` reads them.
- Setting values for fixture jobs; a per-object override of power and speed (refused in concept §1; the grid cells are the one case and run through the grid row).
- Comparing a material across machines (R-MAT-004), importing LightBurn libraries or `.clb` files, sharing materials with other makers, a "material of this brand" catalogue, kerf and focus-height fields, air assist and other machine switches.
- A raster-image test (dither, DPI, line interval); a grid with a third axis (passes) or with a variable cell shape.
- Overwriting the machine profile's maximum feed or power from a test.
- Sync of the library beyond the folder mechanism of ADR 0038 (R-MAT-005 stays as is).
- Changing the page kind later (`0104`).

## Format and ADR

- `format_version`: the project file gains settings registers on operations, the grid's fields (label fields included) and mark, and the material link with its stored numbers. This is new data on existing nodes and possibly one new node shape for the grid row: the architect decides whether it needs a bump from **42** to **43** (the rule that bumped 37 to 41). The material record is a separate file with its own `format_version` 1 and needs no project bump.
- **ADR needed (architect)**, `needs-customer` for the record's location and shape: persistence of material records (per-user `materials/<uuid>.json`, the history, the machine tie), the form of the grid row (Frage 183), the stored numbers beside the material link (Where settings live, 2), and whether the record model stays in `curvyo-laser-core` or moves to its own crate as ADR 0038 anticipates. Proposal: stay in `curvyo-laser-core` (no new crate before a second user).

## Open customer questions

183 to 189 are answered; 190 is open with a default and does not block the build. Numbers continue after `0103` (176 to 178).

**Answered by the customer 2026-11-11 (Fragen 183 to 189):**

**Frage 183 (MAT-GRID-ROW): Answered A.** One folded "Test grid <N>" row for the whole grid; each cell runs with its own power and speed inside it (criterion 6).

**Frage 184 (MAT-HOME): Answered A.** Material records live in the per-user data directory, one file per record, tied to a machine profile by id (criteria 16, 17).

**Frage 185 (MAT-COPY): Answered B.** The operation points to the material by id and does not copy; the numbers stored beside the link are the product owner's Proposal, vetoable at the demo (criteria 24 to 26).

**Frage 186 (MAT-LABEL): Answered.** The label settings are configurable as in LightBurn: Labels on or off, kind (Engrave or Score), height, power, speed and passes in the grid fields (criterion 7).

**Frage 187 (MAT-STORED-REFRESH): Answered B+C.** The stored numbers are also refreshed on every material edit (in the same undo step of the open project) and on every save that changes the project, besides pick and output (criterion 26).

**Frage 188 (MAT-TYPE-UNLINK): Answered B.** A typed number keeps the link and overrides that number, shown as "Differs from <name>" with "Reset to material" (criteria 24, 25).

**Frage 189 (MAT-UNDO): Answered B.** Material edits are undo steps, within the limits of criterion 32 (only this session; New, Open or quit ends it; another project does not take it back; History-list actions restore only the project).

**Frage 190 (MAT-UNDO-HISTORY): Does Ctrl+Z of a material edit also remove the history entry the edit added to the record? Open.**
- A: yes; after Ctrl+Z the record is as before the edit (an undone "Save as material" also takes its test entry back); Redo adds the entry again.
- B: no; the entry stays and an "Undone" entry is added; the history shows every edit, also undone ones, and fills its 500 entries faster.
- Recommendation: A (Ctrl+Z means "as if not done"; the grid and its mark are still in the project, so the test can be saved again). Default if unanswered: A.

## UX notes

`specs/0028-material-test-library/ux-notes.md` (ux-engineer, 2026-11-11): the Add group, the Test grid row, the Grid block with its result table, the Save as material form, the Settings block with the material pick, the Materials section, the canvas look, keyboard routes and the glyphs. Its section 16 lists the decisions M1 to M16 for veto; they are applied in the criteria above.

## Links

Requirements: R-MAT-001, R-MAT-002, R-MAT-003 (Must); R-MFG-001 (machine profile), R-MFG-002 (roles), R-MFG-010 (unknown kinds round-trip), R-SYS-002, R-SYS-007; related, not built here: R-MAT-004, R-MAT-005
Parent: `specs/0063-manufacturing-workbenches/` (slice 6, concept `concept-output-pages.md` §3, §9)
Depends on: `0101` (operations, rows, rename, armed trash, arrows), `0026` and ADR 0038, ADR 0039 (profile and its copy), `0102` and `0103` (fixture jobs stay without settings), `0081` (armed trash), `0098` (laser page)
Next: `0029` `laser-job-preview-and-output` reads the operation's settings (Where settings live, 3)
Changes: `specs/0101-laser-operations/` (criterion 3 and its UX note: several rows selectable), `specs/0103-fixture-jobs/`, `docs/adr/0038-machine-profile-records.md`, `docs/requirements.md` (R-MAT-001 to R-MAT-003 after the demo)
PR: not yet
