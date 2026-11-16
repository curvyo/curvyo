# Laser job preview and output: a toolpath and time preview, a machine and material gate, and GRBL G-code export from a laser page

Status: Draft
Priority: Must
Origin: Customer (R-MFG-003, R-MFG-LASER-001, R-SYS-008; MVP slice 16, the capstone of epic `0063`, "Settings and output"). Everything marked **(Proposal)** is the product owner's and vetoable at the demo. Fragen 219 to 226 are open; each has a default, none blocks the build.

## User value

As a maker with a GRBL laser I want to see on my laser page exactly what will run, in which order and for how long, to be told which machine and which material numbers every step will use before anything is written, and to export one G-code file that cuts, scores and engraves my design, so that a design drawn in Curvyo reaches my laser without another program in between.

**What other tools do.** LightBurn previews in a separate window with a play slider and estimates time; its machine and layer settings are checked by the maker's memory, not by a gate; a wrong device or a layer with the wrong power goes into the file without a word. Inkscape's laser plug-ins write G-code from colours and ask for power in a dialog; they have no preview of order or time. xTool and Glowforge are tied to their own machines. **We do better:** the preview is the page itself (no second window), the gate names the machine and every operation's numbers and where they come from (material, typed, stored) in plain text next to the Export button, what blocks is stated and what only warns is stated, and the numbers are the ones the maker already set in operations and materials, so the export dialog has no settings at all.

## Words (shown in the UI)

- **Output tab:** a tab of a laser page, after Fixtures (`0113`): the gate, the summary and the Export button. Pages of other kinds have none.
- **Job:** everything that runs when the page is output: the running fixture jobs, then the work operations, as `0103` criterion 14 and `0101` criterion 7 define. Nothing else is in a job.
- **Toolpath view:** a toggle at the page name on the canvas (next to the Operations view toggle of `0111`), symbol `Route`, tooltip and accessible name "Toolpath view". Pressed: the page shows the job's moves instead of the drawing's style.
- **Burn move:** a move with the laser on. **Travel move:** a move with the laser off. In the Toolpath view a burn move is drawn by the look of its operation's kind (`0101` criterion 18: Cut red, Score dotted ink, Engrave pale orange), a travel move as a thin dashed ink line, the start of each operation with a numbered ring (ink, the operation's place in the run order) and a small arrowhead at the start of each burn path. Nothing is told by colour alone.
- **Gate:** the block at the top of the Output tab that states the machine and, for every operation that runs, its numbers and where they come from, and lists what blocks and what only warns.
- **Block / Warn:** a **block** is a condition that disables the Export button until the maker removes its cause; a **warn** is shown as a line with a symbol and a text and never disables Export.
- **Effective numbers:** an operation's power, speed, passes (and Interval) after the rule of ADR 0054 §2 and `0028` "Where settings live" 5: the live material's numbers, or the stored ones when the material is not found, with the operation's typed overrides applied over them.
- **Export:** writing the job as one G-code file; it also records that the job was output (criterion 28).

## Acceptance criteria

"Laser page", "item", "leaf", "Not assigned", "nearest rule", "Cuts last" are as in `0101`; "fixture job" and "Runs first" as in `0103`; "operation-only settings" and "Motorised Z axis" as in `0111`; "test grid", "Lasered" as in `0112`; "set", "pin", "Centre mark" as in `0113`. The page's machine is its profile copy (`0039`). All numbers in the criteria are mm unless a unit is named.

### The Output tab and the gate

1. Given a laser page whose extension is on, then it has an Output tab after Fixtures (symbol `FileOutput`, tooltip "Output"); it holds, in this order, the gate, the summary and the Export button (symbol `Download`, tooltip and accessible name "Export G-code"). A page of another kind, and a "Laser (off)" page, have no Output tab. The tab has no setting field of the job; every number comes from operations, materials and the machine profile (Proposal, "no machine settings clutter").
2. Given the gate, then its first line names the machine as "<profile name> · <width> x <height> mm · origin <corner> · GRBL" and its second line the laser mode ("Dynamic power (M4)" when the profile's Laser mode is on, "Constant power (M3)" when off) with the S range ("S 0 to <S-max>"). Below, one line per operation that runs (Output on, a readable kind, at least one item), in run order: "<n> <name> · <material name or Own numbers> · <power> %, <speed> mm/min, <passes> x" plus, for Engrave and Engrave + Outline, "· <interval> mm", and a source word: "live", "stored" or "typed" for each overridden number (the three states of `0028` criterion 26, by symbol and text). A running fixture job shows one line "Fixture jobs · <material name> · <power> %, <speed> mm/min" per kind it runs (Frage 221).
3. Given the gate, then every line is recomputed in the same frame as any change of an operation, a material, the machine or the Output switches, and shows the same numbers the operation's unfolded row shows (`0111` criterion 18). Test: change Cut 1's typed power, the gate line changes at once.
4. Given a condition that blocks (Frage 220 A: **(a)** an operation or fixture job that runs and has no power or no speed ("No settings"); **(b)** burn geometry outside the work area, by the box of the geometry that would run, with a tolerance of 0.01 mm; **(c)** a machine profile copy that is damaged or unreadable; **(d)** nothing at all runs), then the gate shows one line per cause with the symbol `OctagonX` and a text that names it ("Cut 2 has no power and speed.", "3 items lie outside the 400 x 300 mm bed."), the Export button is disabled with its tooltip naming the first cause, and the line's count is a button that selects the offending items or operation. Geometry that would not run (Not assigned, Output off) never blocks.
5. Given a condition that warns (Frage 220 A: Generic laser as the machine; a material not found, using stored numbers; a material with no set for this laser and kind; a typed override of a material number; a speed above the machine's maximum feed or a Z offset with no motorised Z axis (clamped or ignored, criterion 21); items Not assigned; items that run while hidden; operations with Output off; an image skipped (Frage 226); a Cut-out of an item with no closed contour), then the gate shows one line per cause with the symbol `TriangleAlert` and a text, and Export stays enabled. Each count is a button that selects the items. The lines are in a fixed order: blocks first, then warns, each group in the order of this criterion.
6. Given no machine profile is selected on the page (the profile copy is absent), then the page cannot be a laser page (`0098`), so this case is the block (c); given the built-in Generic laser, then it is a profile and warns only (criterion 5; R-SYS-008: the gate states the machine and refuses with none).
7. Given the Output tab with nothing to run, then the gate reads "Nothing runs. Assign items to an operation." (the text of `0101` criterion 17), the summary is empty and Export is disabled (4 d).

### Preview

8. Given a laser page, then the Toolpath view toggle sits at the page name next to the Operations view toggle; the state is per laser page, session state, not saved, not a history step. Pressed, the page draws the job as in Words (Toolpath view) and nothing else of the drawing (the items' own style is hidden, fixtures and the grid stay as built). While pressed the page is read as in `0111` (select, move and the edit tools still work); a change of the drawing or an operation updates the view (Frage 219 A).
9. Given the Toolpath view and a job, then the burn moves are exactly the moves the export writes (one function builds both, so the view and the file cannot differ), drawn in run order with the numbered ring at the start of each operation, an arrowhead at the start of each burn path and the travel moves between them. Test: the sum of the burn and travel lengths read from the view equals the lengths read from the exported file within 0.01 mm.
10. Given the summary in the Output tab, then it reads "Burn <length> · travel <length>" (m with two decimals above 1 m, else mm, whole numbers) and "About <h> h <m> min" ("About 12 min 40 s" below an hour, "About 1 h 5 min" above), one line per operation and a total. The estimate is the burn length divided by the programmed feed, times passes, plus the travel length divided by the machine's maximum feed rate; no acceleration is modelled (Proposal: it runs short, the text says "About"; an acceleration field in the profile is later). The numbers are recomputed in the same frame as the gate. Test: a 100 mm Cut line at 600 mm/min, 2 passes, travel 100 mm at 6,000 mm/min reads "About 21 s" (200 mm / 600 mm/min = 20 s, plus 1 s).
11. Given a hidden item that runs (`0101` criterion 13), then the Toolpath view draws it anyway (what is output is what is shown); given Show off on an operation (`0101` criterion 15), then the Toolpath view still draws its moves (Show is view of the drawing, the preview shows the job).
12. Given the Toolpath view pressed on a page with a job of 1,000 items, then every pointer move (pan, zoom) takes within 16 ms; rebuilding the job after a change is within the budget of criterion 36.

### What each kind writes

13. Given a leaf in a Cut operation, then every outline runs once per pass as burn moves along the flattened curve (criterion 17), a closed outline returning to its first point, an open path along its own direction, as `0101` criterion 19 says; Score the same, in the Marking section; an image in Cut or Score writes its frame as a line (`0101` criterion 19, 20). Fill, stroke, colour and dash never count (`0101` criterion 24).
14. Given a leaf in an Engrave operation, then the closed areas run as parallel burn lines Interval apart (the Interval of the operation, `0111` criterion 19) at the Scan angle (0 degrees: lines parallel to the bed's X axis, positive angles turn the lines counter-clockwise as seen on the page, `0111` criterion 20), clipped to the area by the leaf's fill rule (`0101` criterion 21); an area with holes leaves its holes empty; open outlines of a leaf with a closed outline are closed by a straight line (`0101` criterion 21); a leaf whose outlines are all open runs as lines (140 A). The first line sits half an Interval inside the area's box edge (Proposal).
15. Given Bidirectional fill on (default), then the burn lines alternate direction line by line and the travel between two lines of one area is the shortest connection; off, every line runs in the same direction with a laser-off travel back. Given Crosshatch on, then the area is hatched a second time at the Scan angle plus 90 degrees after the first hatch, in the same operation and pass. Given Overscanning, then each burn line is extended on both ends by the stated percentage of the run-out the speed needs (`0111` Words), with the laser off over the extension, and the laser switches on and off exactly at the area's edge (Proposal for the run-out length: the distance to reach the feed at 1,000 mm/s2, stated in `adrs.md`, because the profile holds no acceleration).
16. Given Engrave + Outline, then the Engrave hatch of criterion 14 runs, then every closed outline as a line (criterion 13), the hatch with the Engrave set and the outline with the Score set (`0028` criterion 25); given Cut-out, then the outer contour of each item runs once per pass as one closed line (`0101` criterion 23, with its margin).
17. Given a curve, then it is written as straight segments with a deviation of at most 0.01 mm from the exact curve (Proposal; no G2 or G3 arcs, so every controller runs the file the same way); a placed page, a clone and an item with a transform write its geometry as it stands on the laser page.
18. Given an image in an Engrave or Engrave + Outline operation, then (Frage 226 A, default) its pixels are not written: Engrave writes nothing for it, Engrave + Outline writes its frame as a line, and the gate warns "N images are not engraved yet." (criterion 5).
19. Given the operation-only settings of `0111` and a Z offset: a Z offset on an operation (only with the motorised Z axis, `0111` criterion 21) is written as an absolute `G1 Z<offset>` move at the operation's start with the laser off and `G1 Z0` after its last pass; with the capability off nothing is written and the gate warns once when an offset is stored (criterion 5). The file never changes Z beyond these two moves.

### Order and passes

20. Given the job, then the order is: the running fixture jobs first (Frage 176 A of `0103`, "Runs first"; inside that block Marking before Cutting while Cuts last is on, list order while off), then the work operations in list order (`0101` criterion 5, 7); an item in several operations runs in each (`0101` criterion 7). Test: Engrave 1, Cut 1 and a fixture job's Score produce, in the file, the Score first, then Engrave 1, then Cut 1.
21. Given the passes of an operation (1 to 20, `0028` criterion 24), then the operation's whole set of paths is written once per pass in the same order (Frage 223 A: pass 1 of all paths, then pass 2); with a speed above the machine's maximum feed rate the written feed is the maximum and the gate warns.
22. Given the order of the paths inside one operation (Frage 225 A), then a path that lies inside the outline of another path of the same operation runs before the path that surrounds it (inner shapes, holes and islands before their outlines, to any depth), and among paths of equal depth the next path is the one whose nearest start point is closest to the head's last position (nearest neighbour; a closed path may start at any of its nodes, an open path at either end); the choice is deterministic: ties go to the lower stacking order. Test: a rectangle with a circular hole and a text inside the hole cuts the text, then the hole, then the rectangle.
23. Given a Cut-out and a Cut in one job with Cuts last on, then the cutting operations run after every marking operation, so no cut loosens a part before it is marked (`0063` criterion 5).

### The file

24. Given a GRBL file, then it holds in this order: a comment header (job name, machine name, the operations in run order with their numbers; no date, no time, no path of the maker, no user name, no application version, no random number, so the same project and settings always write byte-identical text), `G21` (mm), `G90` (absolute), `G17`, the laser-off and S0 start (`M5` then `M4 S0` or `M3 S0` by criterion 2), then the moves, then `M5` and the end (criterion 25). Lines end with LF; coordinates have 3 decimals, S whole numbers, F whole numbers.
25. Given the S value, then S = round(power % x S-max / 100) with the S-max of the machine profile (`0026`); a burn move carries F and S only where they change (`G1 X.. Y.. F.. S..`), a travel move is `G0` with the laser off. Given laser mode on (M4), then S0 may be written in a burn line's end; given M3, travel moves are preceded by `M5` and burn moves by `M3 S<n>`. Air assist is written by Frage 222 (default B). The end of the file: `M5`, optionally `M9`, and a rapid to the job's origin (`G0 X0 Y0`, Proposal: the head returns to where the job started, laser off).
26. Given the origin corner of the machine profile (`0026`), then file coordinates are the page's coordinates measured from that corner with axes pointing into the bed (bottom-left: Y up, as the page; top-left: Y down; right corners: X from the right), so every coordinate of the work area is zero or positive and the file's origin is the corner the machine homes to (Frage 224 A). No homing, no `G10`, no `G92`, no `G54` offsets are written. Test: the same page with the four corners gives four files whose coordinates are mirrored, each starting inside the bed.
27. Given Export, then it works as the SVG export of `0024` criterion 4: the OS save dialog (a download in the browser build) opens with the default name "<project file name without extension> - <page name>.gcode" ("Untitled - Laser 1.gcode"; Proposal; characters not allowed in file names are replaced by "-"), the notice "Exported <name>." (browser: "Downloaded <name>.") shows for 3 s, and a write failure shows the existing error dialog with the system's reason and writes nothing. Cancelling the dialog does nothing and writes no history step (criterion 28).

### After the output

28. Given a successful export, then in one undo step "Export G-code": (a) every fixture job whose Output was on is switched off and shows "Output off" (`0103` criterion 6, Frage 178 B), with the notice "Output turned off on N fixture jobs." when N is above 0; the Centre mark and Diameters jobs of pins and screws (`0113` criterion 29) are such jobs; (b) the stored numbers beside every operation's material link are refreshed, never the overrides (`0028` "Where settings live" 5); (c) every test grid that has at least one operation in the file gets the state **Lasered** in its material (`0112` criterion 11), so a later change of the grid or of its operations by hand gives it a new test id. Undo restores (a) and (b); **Lasered stays** (Proposal: it records that a file was written, not a project state, as `0112` keeps tests outside the project). The Lasered state is set by this export only; the internal test hook of `0112` stays for tests.
29. Given an export that wrote no operation of a grid (its operations had Output off), then that grid is not Lasered and its id stays.
30. Given a project with test grids exported, then the next export of an unchanged grid keeps its id (same grid, nothing changed) and a changed one has the new id (`0112` criterion 11); the file's header names the test id in the label operation's name.

### Files, failures, format and budgets

31. Given the same project and settings, then two exports are byte-identical; golden files in `tests/fixtures/` (`curvyo-laser-core/tests/fixtures/gcode/`): `cut_line.gcode`, `score_and_cut_order.gcode`, `engrave_bidirectional.gcode`, `engrave_crosshatch_scan_angle.gcode`, `engrave_overscan.gcode`, `engrave_outline_set.gcode`, `cutout_margin.gcode`, `inner_first.gcode`, `passes.gcode`, `m3_constant_power.gcode`, `z_offset.gcode`, `origin_corners_bl.gcode`, `_br`, `_tl`, `_tr`, `fixture_jobs_first.gcode`, `image_skipped.gcode`. Each is read back by a tester-written parser that checks the extent, the sum of lengths and the order of operations independently of the generator.
32. Given the whole slice, then **no new format version is needed for the job itself** (the job is derived from the page, nothing of it is stored); the architect confirms in `adrs.md`. Frage 221 B stores a material id per page for fixture jobs, Frage 222 B stores a profile switch; both are small new registers with the format bump the architect assigns (ADR 0054 and ADR 0039/0060 precedents). A file saved before this slice opens unchanged and is not rewritten on open.
33. Given an operation of an Unknown kind, an Unknown fixture job, a damaged `op_settings` or `op_options` (`0111` criterion 25), then it runs nothing, is listed in no gate line as a run and does not block; the damage shows as one warn line "<n> operations have damaged settings and do not run." (at most 70 characters).
34. Given the job builder in `curvyo-laser-core` (no filesystem, network, clock, thread or UI; `#![forbid(unsafe_code)]`; builds for `wasm32-unknown-unknown`), then writing the file and the save dialog are the app's job; this slice adds no crate without an ADR and no dependency (Proposal; the architect confirms).
35. Given the whole slice, then there is no device I/O, no connection to a controller, no streaming, no network access, no account and no telemetry (`R-SYS-002`, `R-SYS-007`): the file is the output, the maker sends it with a program of their own until `0064`.
36. Budgets (thread CPU time, `curvyo-editor-wasm/tests/cpu_time/` and `curvyo-laser-core`; the architect confirms the limits): (a) the job of a page with 1,000 Cut items of 20 nodes each builds within 100 ms; (b) an Engrave of a 200 x 200 mm area at Interval 0.1 mm (2,000 lines, bidirectional) builds within 150 ms; (c) the same area with Crosshatch within 300 ms; (d) the gate and the summary recompute after one edit on a page with 50 operations within 16 ms (the job is built lazily and cached by the page's revision, ADR to confirm); (e) the Output tab opens within 100 ms.

## Changes to accepted specs

| Spec | Criterion | Was | Now |
|---|---|---|---|
| `0101` | 7 (the order inside one cutting operation belongs to `0029`) | not decided | inner first, then nearest start (22) |
| `0101` | 21 (image pixels output by `0029`) | an image runs as raster engrave | pixels are not written in this slice (18; Frage 226) |
| `0103` | 6 (page output turns fixture job Output off) | `0029`'s acceptance | built in 28 |
| `0103` | 14 (run order) and Frage 176 A | built as default | stated in 20 |
| `0112` | 11 (Lasered set only by `0029`; internal hook) | hook only | the export sets it (28) |
| `0113` | 29 (jobs of pins turned off after output) | `0029`'s acceptance | built in 28 |
| `0028` | "Where settings live" 4 and 5 (`0029` decides fixture job numbers; reads effective numbers) | pending | 2 and Frage 221; 28 (b) |
| `0026` | out of scope: enforcing the limits, the gate, G-code | not built | 4, 5, 21, 25, 26; Frage 222 may add one field |
| `docs/requirements.md` | R-MFG-003, R-MFG-LASER-001, R-SYS-008 | Proposal / Customer | status set at acceptance |

## Out of scope

- Sending the job to a machine, streaming, status, pause and stop, framing and homing on a connected laser (`0064`); a frame file or a frame preview on the machine.
- Other controller dialects than GRBL; Marlin, Ruida, LinuxCNC and a dialect choice; a controller field in the profile.
- A focus test, auto-focus, rotary and any second axis; the camera; job start from the head's current position or a user origin (Frage 224).
- Image engraving of pixels, dithering, a power ramp and Offset fill (Frage 226; noted for later in `0063`); Dot mode.
- A simulation with play, pause and a moving head (R-MFG-009), an acceleration model and a more exact time estimate; a 3D view.
- Tabs and bridges for Cut-out, lead-in and lead-out of cuts, kerf compensation, cut order by the maker's drag in the preview.
- Material or operation settings in the export (there is no export dialog); a G-code editor; importing G-code.
- Other machine kinds (cutting plotter, CNC, embroidery); a machine abstraction before the second family (`CLAUDE.md` §6).
- Library or sync of the exported file; remembering the last export folder across machines.

## Open customer questions

Defaults apply until the customer answers; none blocks the build.

**Frage 219 (PREVIEW-SHOW): What does the preview show, and when?** (Criteria 8 to 12.)
- A: the Toolpath view on the canvas with burn moves, travel moves, run order and arrowheads, recomputed live while the maker edits; time and lengths in the Output tab.
- B: the same view, built only when the maker presses a Preview button; it is marked stale as soon as the page changes.
- C: A plus a slider that plays the job along its moves.
- Recommendation: A (what is output is what is shown, no extra button, and the playing view is the later simulation of R-MFG-009).
- Default if unanswered: A.

**Frage 220 (GATE-BLOCKS): What does the gate block, and what does it only warn about?** (Criteria 4 to 7.)
- A: blocks when something that runs has no power or speed, when burn geometry lies outside the bed, when the machine copy is damaged, or when nothing runs; everything else (Generic laser, material not found, typed override, Not assigned, hidden items, images skipped) warns and Export stays on, no confirmation tick.
- B: as A, but every warn line must be ticked "Understood" before Export is enabled.
- C: blocks only no numbers and nothing to run; geometry outside the bed warns.
- Recommendation: A (a file with no power or outside the bed is wrong; the rest is information the maker sees in plain text next to the button).
- Default if unanswered: A.

**Frage 221 (FIXTURE-NUMBERS): With which power and speed does a fixture job run?** (Criterion 2. Fixture jobs have no settings, customer 2026-11-10; `0028` leaves the decision to this story.)
- A: with the numbers of the page's last operation of the same kind; without one the job blocks Export.
- B: one pick in the Output tab, "Fixture jobs run with <material>", stored on the page; each job uses that material's set of its kind for the page's laser; without a pick a running fixture job blocks Export.
- C: the machine profile carries one "Fixture jobs" material that travels with the machine; no pick on the page.
- Recommendation: B (explicit, uses the material library, no new numbers in the job rows, and a bed board is often marked with its own material).
- Default if unanswered: B.

**Frage 222 (AIR-ASSIST): Does the file switch air assist on?** (Criterion 25. The profile has no field for it.)
- A: never; no air code is written.
- B: a machine field "Air assist" (off by default) in the profile next to Laser mode; on writes `M8` before the first burn and `M9` at the end, for every job on that machine.
- C: a switch per operation (Cut on, Engrave off).
- Recommendation: B (air is a property of the machine's setup; a per-operation switch is the settings clutter the customer refused).
- Default if unanswered: B, off by default (the file equals A until the maker turns it on).

**Frage 223 (PASSES): What does "3 passes" do?** (Criterion 21.)
- A: the whole operation runs three times, pass 1 of all paths, then pass 2 of all paths.
- B: each path is run three times before the next path starts.
- Recommendation: A (the material cools between passes, parts stay held until the last pass, and a failed pass can be seen before the next).
- Default if unanswered: A.

**Frage 224 (JOB-START): Where does the job start on the bed?** (Criterion 26.)
- A: at the machine's own origin: file coordinates measured from the profile's origin corner, no homing and no offsets written; the maker homes the machine.
- B: A, plus a switch in the Output tab "Start from: Machine origin / Head position" (job placed where the head stands, the file in relative moves).
- C: A, plus a user origin set on the page (a marked point of the bed).
- Recommendation: A (one start rule for the first file; B and C need the connector `0064` to read the head's position or are a second concept of origin).
- Default if unanswered: A.

**Frage 225 (PATH-ORDER): In which order do the paths of one operation run?** (Criterion 22.)
- A: inner shapes before their outlines, then the nearest next path; no option.
- B: as A, plus a switch "Inner first" in the Output tab, on by default.
- C: the stacking order of the drawing, no optimisation.
- Recommendation: A (inner first is needed for a correct cut, shortest travel is free, and a switch that is never turned off is clutter).
- Default if unanswered: A.

**Frage 226 (IMAGE-ENGRAVE): What does the file do with an image in an Engrave operation?** (Criterion 18. Dithering and a power ramp are later.)
- A: nothing in this story: the pixels are not written, Engrave + Outline writes the frame, and the gate warns.
- B: a plain black and white version: pixels darker than 50 % grey are burned as hatch lines at the operation's Interval (one threshold, no choice).
- C: variable power by grey value along each line (large files).
- Recommendation: A (the customer put dithering and the power ramp in a later story, and a threshold the maker cannot adjust invites bad results).
- Default if unanswered: A.

## UX notes

To be written by the ux-engineer before Ready: the Output tab layout (gate lines with symbol and text, blocks above warns, the Export button below the summary and its disabled tooltip), the Toolpath view look (Words: the numbered ring, the arrowhead, the dashed travel line; the existing red, pale orange and ink only, no new colour), the `Route` toggle beside `ListTree`, the empty state, the keyboard route and accessible names. Rules from the customer lessons: no invented colours (symbols and text), symbol-only buttons with a tooltip, units inside the fields, plain fields not sliders, tool-specific bars, nothing like LightBurn's layer table, no machine settings in the operations. The tab has no field except Frage 221 B's single material pick.

## Links

Requirements: R-MFG-003, R-MFG-LASER-001, R-SYS-008, R-MFG-005, R-MFG-009 (later), R-SYS-002, R-SYS-007
Parent: `specs/0063-manufacturing-workbenches/` (slice 6, "Settings and output")
Depends on: `0101`, `0102`, `0103`, `0111`, `0112`, `0113`, `0028`, `0026`, `0039`, `0038`, `0060`, ADR 0046, ADR 0052, ADR 0054, ADR 0055
Next: `0064` `machine-connector` (sending, framing, streaming); a later story for image engraving
Changes: `specs/0101-laser-operations/`, `specs/0103-fixture-jobs/`, `specs/0112-test-grid-and-materials/`, `specs/0113-fixture-sets-and-pins/`, `specs/0028-material-test-library/`
Format: none for the job (criterion 32); Frage 221 B and 222 B each add one small register, numbers assigned by the architect at merge. Goldens: `curvyo-laser-core/tests/fixtures/gcode/` (criterion 31).
PR: -
