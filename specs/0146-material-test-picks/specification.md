# Material test picks: Best, Second best, Fastest and Deepest cells of a test, and round Engrave test marks

Status: Done (PR #517; Fragen 312 to 316 built as default A, not yet answered; not built: 1 px hatch edge on circle cells, folded pick lines for a kind without a main set, dark token values)
Priority: Should
Origin: Customer (2026-11-20, translated, with a screenshot of xTool's material test: a speed x power grid of small marks on a plywood board, cells framed and labelled "Beste" in green (several cells), "Schnellste" in blue (one cell), "Tiefste" in orange (one cell)): the materials need more fields: the "best" setting exists, add "fastest" and "deepest" and allow a second choice (2nd best) too, as in the screenshot; and in the Engrave test grid every test mark should be an engraved circle instead of xTool's X. Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

Revises `0112` (Done, PR #353; criteria 23, 24, 12 and the cell look), extends `0028` criterion 24 to 27 (the operation's material pick) and rides the sync of `0116` unchanged in concept. Nothing here is built.

## User value

As a maker who has burned a test grid on a new sheet I want to mark not only the one best cell but also the second best, the fastest cell that still looks right and the deepest cell, and later choose which of the four a job uses, so that a quick draft job runs the fastest numbers, a deep relief runs the deepest, and my second choice is there when the best one is too strong for a thin sheet.

**What other tools do.** xTool Studio marks Best, Fastest and Deepest cells on its material test and keeps them in its material library, with several cells allowed per mark. LightBurn's Material Test generates the grid and keeps nothing of the result. We do the same picks, keep them per test and per laser in the shared material, and let each operation say which pick it follows.

## What exists today (reading of `0112`, ADR 0057, ADR 0058, 2026-11-20)

- A test (record in the material) holds one `best` cell (`column`, `row`). Tapping a cell in the test detail writes `best` and the laser's **set** of that kind (power, speed, passes; Engrave also interval) from that cell, source "Test <id>", one step "Mark pick" (was "Choose best cell"). A tap on another cell moves the mark and the set.
- A material has per laser and per kind exactly one set; an operation that points to the material takes it (`0028` criteria 24 to 27, `0112` criteria 18 to 21). Image has its own set (`0123`).
- Cells of a grid are items: Cut and Score cells are closed squares as outlines, Engrave cells are filled squares (hatched when engraved), Image cells are gradient rectangles. A cell has its own derived operation.

## Words (shown in the UI)

- **Pick:** a cell of a test that the maker marked with one of four **pick kinds**.
- **Best:** the main choice (as today; word kept: "Best cell"). **Second best:** the fallback choice. **Fastest:** the cell with the highest speed that still gives an acceptable result. **Deepest:** the cell that cuts or engraves the deepest or darkest. All four are the maker's judgement; Curvyo never computes them.
- **Pick set:** the set (power, speed, passes; Engrave also interval) that a pick copies from its cell, as the Best cell does today.

## Acceptance criteria

"Laser page", "set" and "test detail" are as in `0112`.

### Marking picks

1. Given a test detail (`0112` criterion 23), then above the table sits one **pick bar**: four toggle buttons in a 2 x 2 grid (Best, Second best / Fastest, Deepest; 228 px wide, a row does not fit the column), symbol and word each (Best, Second best, Fastest, Deepest), exactly one pressed at a time, Best pressed when the detail opens (Proposal; inline, no popup, no menu). The table's accessible description names the pressed kind. The buttons are `aria-pressed` buttons with manual activation: arrow keys move focus only and never mark or write a step.
2. Given a pressed kind, when the maker taps a cell, then that cell carries that kind and the kind's pick set is written from the cell (the rule of `0112` criterion 24: the two axis values plus the shared values); one undo step "Mark pick" for all four kinds (renames today's "Choose best cell"; the internal key `judge_test` stays). Each kind marks **at most one cell per test** (Proposal, Frage 312 A): a tap on another cell moves the kind's mark and its pick set. One cell can carry several kinds (the fastest cell may also be the best).
3. Given a cell that already carries the pressed kind, when the maker taps it again, then the kind is removed from the cell, one undo step "Unmark pick" (Proposal). The set is touched only if it came from this test (its source is "Test <id>" of this test): then, for Second best, Fastest and Deepest, the pick set is removed with the mark, and for Best the laser's set stays as it is and its source reads "Typed" (a set needs no test, `0112` criterion 20; nothing the maker relies on disappears). A set that was taken from another test of the same laser and kind stays untouched, and the mark is removed alone.
4. Given the table, when the maker uses the keyboard, then the cells keep the roving focus of `0112`; Enter or Space on a focused cell acts as a tap, and the pick bar is reached by Tab before the table; no pick kind needs a mouse.
5. Given a test that is not Lasered (`0029` sets that), then picks can be marked as well, as for Best today (`0112` criterion 24).

### Showing picks

6. Given a marked cell, then the table shows it with a frame and a **mark per kind that is not colour alone**: a distinct glyph in the cell, no word or initial (the word is in the pick bar, the cell's tooltip and accessible name, and the pick lines) (Best: check; Second best: check with a small "2"; Fastest: the speed (chevron) glyph; Deepest: the down-arrow glyph) and the frame in the kind's colour; a cell with several kinds shows the glyphs in fixed corners (Best top left, Second best top right, Fastest bottom left, Deepest bottom right; 8 px each in cells of 24 px) and, in narrow cells of 9 to 12 columns, the first kind's glyph only (further kinds in tooltip, name and lines), with a frame in the colour of the first kind in the order Best, Second best, Fastest, Deepest. Frames use 2 px; every symbol and frame reaches at least 3:1 against the table ground and 4.5:1 for text (the UX engineer pins the tokens, see "Colours").
7. Given a test with picks, then the line under the table lists every pick on its own line, as the Best line does today: "Best: Cut set of Diode 20 W: 60 %, 1950 mm/min, 1 ×", "Second best: 50 %, 1800 mm/min, 1 ×", "Fastest: ...", "Deepest: ..." (kinds without a pick are not listed, no hint text).
8. Given the grid of a test that is on a laser page and selected, then its cells carry the same frames and symbols on the canvas as a view-only overlay (Proposal): they are never output, never exported, never part of the operations, and disappear when the grid is not selected.
9. Given a material in the Materials tab, then under each laser's set of a kind the other picks of that kind are shown as up to three folded-in lines (Second best, Fastest, Deepest) with the kind's symbol, word, numbers and source "Test <id>"; they are not editable by typing (Proposal: change a pick by marking another cell, the one route keeps picks truthful) and have no add button.

### Storage, old records, sync

10. Given a pick, then it is stored on its test (the cell per kind) and its pick set is stored in the material, per laser and per kind of the test, next to the main set, so a pick still works after the test is deleted: its source reads "Test <id>" with the id of the deleted test, and the record stays valid (as `0112` criterion 25 says for Best; today a record whose source names a deleted test reads as damaged, which ADR 0092 decision 4 fixes for all kinds, best shipped ahead as a `fix/` PR for format 4). Nothing else is stored; no operation-only setting (Crosshatch, Bidirectional fill, Scan angle, Overscanning, Z offset) is copied (`0112` criterion 19).
11. Given a material record written before this story (Best only), then it opens unchanged, shows its Best as today and has no other picks; nothing is converted or rewritten on read (reads never write). A record written with picks and opened by an older Curvyo is left untouched by the "newer format" rule (`0112` criterion 30). The material record becomes **format 5** (ADR 0092) and the record size limit rises from 512 KiB to 640 KiB so that a record with every mark and pick set still fits. Golden files for the new record format in `curvyo-laser-core/tests/fixtures/materials/` (a record with all four kinds on three cells, one cell with two kinds, a pick whose test was deleted, an Engrave pick with interval). Shape and rules: ADR 0092.
12. Given a synced material (`0116`), when picks are marked on two devices, then the record converges as any material edit does (`0116` criteria 7 to 9, ADR 0062 §5): the newer `modified` wins and the other version is kept as a conflict copy with its notice; no new sync mechanism. Picks of different kinds or lasers marked on two devices are **not** merged field by field (that would be a sync decision of its own, ADR 0004 §8, out of scope).

### Applying a pick

13. Given a selected operation that points to a material (`0028` criterion 25), then its settings block shows, next to the material pick, the **pick pick**: the same four symbol-and-word buttons (Proposal for the name: "Use"), Best pressed by default. An Engrave + Outline operation takes the pick for both parts: the Score set of that pick for the outline, else the Engrave set of that pick (Proposal; today's fallback applied per pick). Image tests get picks like the other kinds; an Image pick set holds power, speed, passes and interval, and a picked cell of a picture grid writes only these (ADR 0072 §6) (Proposal). Choosing a kind stores the link with that pick kind and, as the material pick does, the numbers as last seen, in one undo step "Use pick" (the key of ADR 0092); the three states of `0028` criterion 26 (live, material missing, set missing) stay and say the kind ("Material has no Fastest set for this laser, using stored numbers.").
14. Given an operation linked to a pick kind, when the maker edits or re-judges the material, then it follows in the same frame like a Best link (`0028` criterion 26 (a)); typing a number keeps the link and stores an override (`0028` criterion 24, Frage 188 B); a pick of the material, also of the kind already set, clears the overrides.
15. Given an operation with no pick kind stored (every operation made before this story), then it follows Best, exactly as today; nothing in an old project changes. Given a pick kind that the material has no set for (for example Fastest was never marked), then its button is dimmed with the tooltip "No Fastest set for this laser" (not hidden: the maker sees why) and cannot be pressed. Given a selection of several operation rows, then the block shows the material pick and the pick pick, and applies both to all in one undo step; an operation without that set is skipped and named in a notice of at most 70 characters (`0028` criterion 27).
16. Given a derived cell operation of a test grid (`0112` criterion 12), then it keeps its own numbers from the recipe; the pick pick is not offered on it (its numbers are the test, not a material choice).
17. Given the project file, then the operation's link gains `op_settings.material.pick` (absent means Best) and the project `format_version` rises to the next free number at merge (65 today, ADR 0092 decision 7); an old project opens unchanged.

### The round Engrave mark

18. Given an Engrave test grid (`0112`), then each cell is a **closed circle** of diameter equal to the grid's Cell size, centred on the cell's square (Frage 314, Proposal), filled with the Engrave hatch as an Engrave cell is today; the grid pitch (Cell size plus the constant 2 mm gap) and the labels do not change. Cut and Score cells stay closed squares: a cut-through cell drops out whole and the corner marks of a Score square read better (Frage 315, Proposal). An Image grid's cells (`0123`) stay gradient rectangles.
19. Given an Engrave grid with Crosshatch on, then a circle is hatched twice at right angles as a square is; with Crosshatch off once; the hatch is clipped to the circle by the existing Engrave fill (`0101`), so no new output code is written for the mark.
20. Given the circle, then it is the existing ellipse primitive with equal radii (Cell size / 2), not a new path (ADR 0092 decision 10), and every point lies within 0.03 % of the radius of the exact circle (at most 0.007 mm at Cell size 50 mm; the job flattens at 0.0095 mm; four kappa arcs alone cannot keep 0.001 mm above Cell size 7.3 mm) and its area is within 0.1 % of the exact area (explicit geometric tolerances, tested at Cell sizes 3, 10 and 50 mm). Golden test: the items of a 3 x 3 Engrave grid at Cell size 10 mm in `tests/fixtures/` (the nine circles, positions and the unchanged labels).
21. Given a project opened with an Engrave grid made before this story (square cells), then it opens as it was; the cells become circles only at the next committed bar change of that grid, when the layout is regenerated (`0112` criterion 14), in the same step as that change. No notice, no automatic conversion on open (Proposal). Given a Lasered grid, then the change of criterion 11 of `0112` (new test id) applies as before; changing nothing but opening it never changes the id.
22. Given the cell look in the Operations tab and on the canvas, then an Engrave cell reads "circle" through its shape only; no word, tooltip or setting names it and there is no option to switch back to squares (no speculative option; Frage 314 B asks for one).

### Failures, budgets

23. Given a test whose recipe or picks are damaged (a pick cell outside the recipe's counts, a pick set on the wrong kind, a missing `interval_mm` on an Engrave or Image pick), then the record is read as damaged by the rules of ADR 0058 §6 as amended by ADR 0092 decision 5 (named, skipped, never written), with golden files for each.
24. Given a 12 x 12 Engrave grid, then placing it takes within 50 ms and a bar edit within 30 ms with circles (thread CPU time, `curvyo-editor-wasm/tests/cpu_time/`, as `0112` criterion 32); marking a pick is one step within 30 ms.
25. Given the whole slice, then no network access, no device I/O, no G-code change and no telemetry.

## Colours

The screenshot colours (Best green, Fastest blue, Deepest orange) cannot be taken as they are: the design system's accent blue means "selected" (`--accent`), orange is the Engrave hatch (`--engrave-hatch`) and red is Cut. The UX engineer adds four pick tokens to `docs/design-system.md` with measured contrast (Proposal: Best green, Second best a paler green or the same green dashed, Fastest violet or teal, Deepest brown or magenta; none equal to `--accent`, `--cut` or `--engrave-hatch`), in light and dark. Symbol and word carry the meaning; colour only helps (criterion 6).

## Out of scope

- Suggesting a pick: marking the fastest or deepest cell by computation (for Engrave, depth from power, speed, passes; for Cut, a cut-through check from a photo), a smart pick, ranking by photo.
- More than one cell per kind (xTool's several greens), a ranked list of Best (Frage 312 B asks for it), more than four kinds (for example "Cleanest edge"), a custom kind.
- Typed Second best, Fastest or Deepest sets without a test (criterion 9), editing a pick set by typing.
- Marking picks by clicking a cell on the canvas (the table is the one place, as for Best), picks in an exported or printed test sheet.
- A circle mark for Cut, Score or Image cells, a mark shape option, an X mark (Frage 315, 314 B).
- Applying a pick to a whole page or to all operations at once beyond the multi-row selection of criterion 15.
- Converting old square Engrave grids on open (criterion 21).
- Merging picks of different kinds or lasers field by field in sync (criterion 12); that needs a sync decision of its own (ADR 0004 §8).

## Questions for the customer

**Frage 312 (PICK-KINDS): How many cells can carry each pick?**
- A: Four kinds (Best, Second best, Fastest, Deepest), at most one cell per kind per test; several kinds can sit on different cells (so a screenshot like xTool's with several green cells is a Best and a Second best).
- B: As xTool: any number of cells per kind; Best is a ranked list, the first is the set that operations use.
- C: A, but several cells allowed for Best only (unranked, the first marked is the set operations use).
Recommendation: A (one cell per kind keeps the operation's link unambiguous). Default: A.

**Frage 313 (PICK-APPLY): How does a job use a pick?**
- A: Each operation linked to a material says which of the four it follows (criterion 13), default Best; the link stays live.
- B: The picks are only copied as typed numbers into an operation ("Use Fastest" fills the fields and drops the link).
- C: Only Best is linkable; Second best, Fastest and Deepest are shown in the material and copied as in B.
Recommendation: A (one mechanism, old operations unchanged; needs a small project-format change, ADR). Default: A.

**Frage 314 (CIRCLE-MARK): The Engrave test mark.** Today an Engrave cell is a filled square (xTool draws an X). Do you want:
- A: A filled, hatched circle of the cell's diameter instead of the square (criteria 18 to 20); old grids keep their squares until edited.
- B: Both, a Mark switch in the Test grid bar (Circle, Square).
- C: An engraved circle outline only (a ring), no fill.
Recommendation: A (the circle is round at any Cell size, burns a clear dot area for judging depth; a ring shows less of the engraving's depth). Default: A.

**Frage 315 (CUT-MARK): Does the Cut grid keep its own mark?**
- A: Yes, Cut and Score keep closed squares (a cut-through circle would also drop out whole, but the square's corners make a missed cut easier to see).
- B: All kinds get circles.
Recommendation: A. Default: A.

**Frage 316 (PICK-CANVAS): Show picks on the canvas too?**
- A: Yes, view-only frames on the selected grid (criterion 8).
- B: Only in the test detail table.
Recommendation: A (it matches the board photo you showed and helps read the sheet next to the laser). Default: A.

## UX notes

See [ux-notes.md](ux-notes.md) (pick bar and pick pick, glyphs, pick tokens, folded lines, canvas overlay, circle on screen, keyboard, undo texts). M1 to M4 are applied to criteria 1, 2, 6 and 13.

## Links

Requirements: R-MAT-006 (new), R-MAT-002, R-MAT-003
Depends on: `0112` (Done, PR #353), `0028`, `0116`, `0123` (Image set, Image grid), `0101`
Related: `0029` (Lasered), `0135`
ADRs: [ADR 0092](../../docs/adr/0090-material-test-picks-and-round-engrave-cells.md) (Proposed, needs-customer, accepted with the story PR; amends 0054, 0057, 0058). UX: [ux-notes.md](ux-notes.md).
PR: #517
