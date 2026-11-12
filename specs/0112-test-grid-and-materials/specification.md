# Test grid and materials: a Test grid tool that resolves into ordinary operations, a Materials tab with one material shared by all lasers, tests judged and kept by id

Status: Draft (Ready after the customer merges this PR)
Priority: Must
Origin: Customer (laser redesign after the demo of PR #333, answers to Fragen 192 to 211, 2026-11-12; decisions in `docs/design-drafts/2026-11-laser-redesign/README.md`). Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

Second of three slices of the redesign. Builds after `0111` `laser-operation-tools` (operation rows unfold in place, operation-only settings, the Select-bar kind buttons). Followed by `0113` `fixture-sets-and-pins`. Screens: `docs/design-drafts/2026-11-laser-redesign/` 02 (tool bar), 03 (grid as operation rows), 04 (Materials tab), 05 (test detail). It rebuilds the surface of `0028` (Done, PR #333); the numbers, the link of an operation to a material and the history stay.

## User value

As a maker with a new sheet of material I want to place a test grid from one tool, laser it, tap the best cell in the Materials tab and keep that for every later job, with the test's id, a note and a photo, so that I never guess power and speed again, and I find the sheet's test again by the 5-character id I burned into it.

**What other tools do.** LightBurn's Material Test is a generator dialog; its result is not recorded and its library is per layer and per machine setup. We do better: the grid is an object that becomes plain operations, the test is kept by id with note and photo, and one material serves every laser of the maker with its own sets per laser.

## Words (shown in the UI)

- **Test grid:** one object on a laser page, a tool next to Offset and Measure. It is edited any time and resolves into ordinary operations.
- **Test id:** 5 characters `0-9A-Z`, burned into the sheet next to the grid.
- **Test:** the record in the material of one grid that has a material: id, kind, axes, laser, note, photo, state.
- **Lasered:** state set only by the output of `0029`. Until then every test reads "Not lasered".
- **Best cell:** the one cell the maker taps in the test; its values become that laser's set of that kind in the material. (Replaces "Worked" of `0028`.)
- **Set:** power, speed, passes (Engrave also interval) of one kind for one laser in one material. Nothing else.
- **Material:** name, thickness, sets per laser, tests. One record for all lasers.

## Acceptance criteria

"Laser page" is as in `0028`. The page's laser is its machine profile copy (`0039`).

### Test grid tool

1. Given a laser page, then the tool card of Offset and Measure has a third tool, Test grid. Its bar replaces the Select bar like the Trace bar does. On a page that is not a laser page the tool is dimmed with the tooltip "Laser pages only." (the rail's fixed cards dim, they do not remove).
2. Given the tool, when the maker presses on the page, then one test grid is placed with the bar's values where the user pressed, with the first free spot of `0028` criterion 3 as the fallback when the press spot does not fit (one undo step "Add test grid") and selected; the bar then edits the selected grid live, one undo step per committed value.
3. Given the bar, then row 1 holds, without captions: Kind (Cut, Score, Engrave), the test id (tooltip "Test id", read-only), the material pick, the parameters that sit on no axis and are shared by every cell (for example Passes, or Speed for an Engrave test), and Cell size (mm, 3 to 50, default 10). For Engrave it also holds the Crosshatch toggle (`#`, tooltip "Crosshatch"). Row 2 holds Columns and Rows, each one unit of caption, Parameter, Min, Max, Count (2 to 12, default 5). The gap between cells is 2 mm, constant, no field.
4. Given a parameter switch of an axis, then Cut and Score offer Power, Speed and Passes, Engrave also offers Interval; every parameter is offered in both switches; choosing the parameter of the other axis swaps the two axes; the parameter on no axis moves to row 1. Plain number fields with the unit inside; no sliders.
5. Given a value outside its range, or a grid wider or higher than the bed, then the value is kept and marked, the grid stays as it was and a text names the cause ("Grid is 260 mm wide, the bed is 200 mm.") as `0028` criterion 2 says; nothing is clamped.
6. Given the material pick, then it lists the materials, "New material" first (it opens the Materials tab on the new-material form, criterion 17), as a dropdown under the bar (Proposal, the one popup of the tool bars). Min and Max default (Proposal): 20 to 100 % power, 10 % to 100 % of the machine's maximum feed, 0.05 to 0.25 mm interval.
7. Given the grid on the page, then it moves like any object (drag, arrows, Move chip), shows a dashed box and has no resize handles; its size follows Cell size and Count only.
8. Given a copy of a grid (Duplicate, Paste), then the copy is a new grid with a new test id (criterion 11) and its own operations; given a deleted grid, then so are its operations (criterion 13).

### Test id and the burned label

9. Given a new grid, then it gets a test id: 5 random characters of `0-9A-Z`, unique against every test id in the material store, deleted tests included (their ids stay reserved as tombstones, criterion 25). Two computers that assign the same id before they sync are accepted (customer); the later one keeps its id and both tests stay.
10. Given a grid, then the test id, the axis titles and the axis values are always burned: they are engraved, one Engrave operation, there is no option to leave them out and no label settings in the bar. The label operation takes the Engrave set of the picked material for the page's laser, else 20 % power and 3000 mm/min, text height 3 mm (Proposal; it is an ordinary operation, criterion 12).
11. Given a grid whose state is Lasered (set by `0029` only; there is no button for it, an internal test hook sets it for the tests), when the maker changes a grid field in the bar or edits one of its operations by hand (settings, material, options or membership; not rename, Show, Output or order, because what was burned does not change), then the grid gets a new test id at once (the id in row 1 changes, no notice) and its old test stays as it was. Given a grid that is not Lasered, then the same changes keep the id.
12. Given the grid, then it resolves into ordinary operations in the Operations tab: for a 5 x 5 grid 26 operations, one for all text including the id and 25 for the cells, named "<column value> · <row value> · <id>" and "<id> labels" (Proposal), each with its own values; an unfolded row shows a source line (material, test, cell). They are ordinary rows (Show, Output, rename, trash, arrows) and can be edited by hand.
13. Given a deleted grid, or a grid moved off the laser page, then its operations are removed in the same step; there is no "remove operations" button. The test stays in the material.
14. Given a change in the bar, then the grid's operations are regenerated and overwrite hand edits made to them; one undo step restores them.
15. Given the grid, then its operations live in the project file as ordinary operations plus the grid object (the architect decides the shape and whether `format_version` 43 changes).

### Materials tab

16. Given a laser page, then it has a tab Materials after Operations; the materials list leaves the Operations tab. (The Fixtures tab is `0113`.) The Operations tab keeps the material pick of an operation (`0028` criteria 24 to 27) unchanged.
17. Given the Materials tab, then it lists the materials as a flat dense list (name, thickness, "<n> lasers · <n> sets · <n> tests"). "New material" is the `0045` inline form (Name selected, Thickness, Cancel, Add; thickness 0.1 to 100). A material row selects and unfolds in place; rename by double-click or F2; delete with the armed trash (`0081`); order by arrows and drag as `0028` criterion 22. Deleting a material removes its tests, photos and sets; its tests' ids stay reserved.
18. Given a material, then it is one record for all lasers. Under it, each laser has its own sets, one per kind: the page's laser first and open ("This page"), other lasers folded with their set count, "+ Sets for another laser" lists the library's other machine profiles in place, without changing the page's profile (a pick adds that laser to the material).
19. Given a set, then it holds power (1 to 100 %, one decimal), speed (mm/min, whole, at least 1), passes (1 to 20) and, for Engrave, interval (mm), and nothing else; Crosshatch, Bidirectional fill, Scan angle, Overscanning and Z offset are operation settings (`0111`) and are neither stored in nor copied from a material (amends Frage 194 A). Its source reads "Test <id>" or "Typed".
20. Given a kind with no set for the laser, then one "+ <kind> set" button per missing kind (glyph and word) adds three empty fields (four for Engrave) and the source "Typed"; a set needs no test. A set edit is one undo step; the history keeps the old numbers (ADR 0055).
21. Given a laser that is not in the material yet, when the maker adds a set for it, then the laser appears in the material; given a material shared across lasers, then an operation on a page uses the set of the page's laser; without such a set it shows "Material has no <kind> set for this laser, using stored numbers" (`0028` criterion 26 (b), reworded).

### Tests, judging, note and photo

22. Given a material, then "Tests (n)" lists its tests by id, newest first, with state in line 2 ("Lasered 12 Nov · Diode 20 W" or "Not lasered · Birch sheet" with the page name while on a page, or "not on a page") and a photo glyph when it has a photo. A grid gets its test record in the material the moment a material is picked in the bar (Proposal; a grid without material has an id but no test yet); the record then follows the grid (each bar commit rewrites its recipe in the same step) until the test is Lasered, and a pick of another material moves it without retiring the id.
23. Given a test row, when the maker presses it, then the test opens in place of the material body (back button with the material's name): id, kind and laser, the axes line, the date, the table of the cells with the two axes, Note, Photo and the button "Place on <page>" (or "Select on page" while the grid is on a page).
24. Given the table, when the maker taps one cell, then that cell carries a check and an accent frame, and its values become that laser's set of this kind in this material, shown in the line under the table ("Cut set of Diode 20 W: 60 %, 1950 mm/min, 1 ×"); source "Test <id>"; one undo step. A tap on another cell moves the mark and the set. General rule: the cell's two axis values plus the shared values of row 1 of the test (for Engrave with axes power and speed, interval comes from row 1). Judging is possible whether or not the test is Lasered; there is no stepper and no Judge screen.
25. Given the test detail, then a test can be deleted with the armed trash; its id stays reserved (tombstone) so it is never assigned again, and its photo is removed (Proposal).
26. Given the Note field, then it holds one plain text of at most 500 characters, saved on leaving the field, one undo step.
27. Given the Photo block, then one photo per test: from a file or from the browser camera ("Camera" is not shown without a camera or permission), then Crop (frame with eight handles, outside dimmed; Crop writes the cut image, Cancel or Escape keeps the old one; one undo step). The camera image and any file are encoded as JPEG unless the file is PNG, at most 600 px on the long side and at most 1 MiB after encoding, kept next to the material record; a photo is removed with its own trash or with the test.
28. Given "Place on <page>", when the grid is not on a page, then it is recreated on the active laser page with the same test id and the same fields, its operations regenerated, one undo step; the id is the same because nothing changed. Given the test is Lasered, then a later change of the grid gives it a new id (criterion 11) and this test keeps its record.

### Files, failures, budgets

29. Given a project with test grids and operations, when saved and reopened, then all is as before (golden project in `tests/fixtures/`). A project of format 43 or 44 with the old folded grid row (`op_grid`) is converted when the file opens, in one step that is not an undo step and is not saved until the maker saves: the grid becomes an object with a new id and its operations resolved, nothing lost; cells are not re-laid out, so hand-moved cells keep their place and numbers; the burned id label appears at the first bar change; the old mark becomes the best cell of the grid's first test when that test is made.
30. Given a material record of record format 1 (one machine, `0028`), then it opens: the record is read as a material with one laser, its tests keep their history, and it is written back in the new record format when the maker changes it (Proposal; reads never write, the rule of ADR 0038). A record of a newer format is left untouched. Golden files for the old and the new record format in `curvyo-laser-core/tests/fixtures/materials/`.
31. Given photos and ids, then they sync with the record by the folder mechanism of ADR 0038 (no new sync); a photo that is missing or damaged shows "Photo not found" and the test stays usable.
32. Given a 12 x 12 grid, then placing it takes within 50 ms and a bar edit within 30 ms; the Materials list with 200 materials opens within 50 ms (thread CPU time, `curvyo-editor-wasm/tests/cpu_time/`; the architect confirms).
33. Given the whole slice, then no network access, no device I/O, no G-code and no telemetry; the camera stays inside the browser and an image never leaves the machine.

## Changes to accepted specs

| Spec | Criterion | Was | Now |
|---|---|---|---|
| `0028` | Words: Test grid, Worked, Material | grid with columns power and rows speed; a "Worked" cell marked on the grid; a material tied to one machine | tool with any two parameters; "Best cell" tapped in Materials; one material for all lasers (1, 3, 4, 18, 24) |
| `0028` | 1 to 3, "Where settings live" (a Test grid button in the Operations tab) | a button beside the kind buttons, fields in the Properties panel | the Test grid tool and its bar (1 to 7); the button leaves the Operations tab |
| `0028` | 4 to 7 (grid of ordinary items, one folded "Test grid <N>" row, label fields) | one folded row, Labels switch, kind, height, power, speed, passes | 26 ordinary operations (12); test id, axis titles and values always burned, no label fields, gap constant 2 mm (3, 10) |
| `0028` | 8 to 11 (edit like a rectangle, "Edited by hand", Rebuild) | Properties panel, Rebuild button | the bar edits live, regenerated operations overwrite hand edits (Frage 204 A), no Rebuild (14) |
| `0028` | 12 to 14 (result table on the grid, Worked, "Save as material") | marked on the grid, saved with a form | judged in Materials, one tap, no form (24) |
| `0028` | 15, 16 (record: one machine, kind sets of power, speed, passes) | per machine | one material for all lasers, sets per laser, interval for Engrave; record format 2 (18, 19, 30); architect decides |
| `0028` | 19 to 23 (Materials section in the Operations tab, "Show all machines" out of scope) | collapsed section | own Materials tab, flat list (16, 17) |
| `0028` | 28 (save and reopen, grid mark) | marks on the grid | tests and the best cell in the material; `op_grid` resolved (29); format change is the architect's call |
| ADR 0054 | folded grid row, `op_grid` | one row for the grid | derived ordinary operations and a grid object (12, 15) |
| ADR 0055 | record `machine`, `sets` per kind, `history`; remembered grid values | one machine per record | shared across lasers (record format 2), photos next to the record, test records and tombstoned ids, the remembered grid values per machine stay (Proposal) |
| `0111` | draft screens 02, 04, 05 | drafts | built here |

## Out of scope

- The Fixtures tab, fixture sets, pins, centre mark, job fixtures (`0113`).
- The select-bar kind buttons, groups, the operation settings themselves (`0111`).
- Setting "Lasered", output, preview, G-code (`0029`).
- A dithered power ramp and a rectangle with a gradient, also for image engraving (a future story); focus test; back-side aid.
- Comparing materials across machines, importing LightBurn libraries, sharing materials with others, more than one photo or note per test.
- Resize handles on the grid; a Burn-id option or label options.

## Format and ADR

- `format_version` 45 (ADR 0057; the number is taken at merge, after `0111` 44): the project file changes for the grid object and derived operations; material record format 2, `test-grid-choice.json` format 2, new `retired-test-ids.json` format 1 (ADR 0058).
- **Architect:** done in ADR 0057 (grid object, derived operations, migration) and ADR 0058 (material records shared across lasers, photos, retired ids); both `needs-customer`, accepted with the story PR.

## Open customer questions

None. Everything not decided by the customer is a marked Proposal with a safe default (test record created when a material is picked, photo deleted with its test, 500 characters of note, JPEG/PNG at 600 px).

## UX notes

`ux-notes.md` is complete; draft screens `docs/design-drafts/2026-11-laser-redesign/` 02 to 05. Also: the Select tool and the kind buttons on a grid (notes G3, G4), the set trash (G11), arrows on material rows (G9).

## Links

Requirements: R-MAT-001 to R-MAT-003
Depends on: `0111` `laser-operation-tools`, `0028` (Done, PR #333), `0101`, `0045`, `0081`, `0039`
ADRs: `adrs.md`, ADR 0057, ADR 0058. UX: `ux-notes.md`.
Next: `0113` `fixture-sets-and-pins`; `0029` sets "Lasered"
PR: not yet
