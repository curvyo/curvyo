# Cutting plotter: a plotter output page with mats, Cut and Draw, Design Space SVG and HPGL

Status: Ready (slice 1 MVP; later slices stay Draft until their criteria are written) (2026-11-27; `adrs.md` and `ux-notes.md` in, UX mismatches applied as Proposals; earlier: Draft (2026-11-27; customer answers to Fragen 416 to 420 and 427 in; Fragen 428 A and 429 A (for now) answered 2026-11-27; waits for `adrs.md` (ADR 0097 written) and UX notes))
Priority: Should
Origin: Customer request 2026-11-27 (translated): three new machine families, cutting plotter first, built one after the other on `story/machine-families`, one PR at the end. Customer change of 2026-11-27: **first a thin end-to-end slice ("Durchstich") through all three families, then everything else.** Plan and answers: `docs/design-drafts/2026-11-machine-families/plan.md`. Everything marked **(Proposal)** is the product owner's and vetoable at the demo.
Depends on: [ADR 0097](../../docs/adr/0097-cutting-plotter-embroidery-and-cnc-families.md) (record envelope, library merge, the profile record's `kind` per family, materials per family in `curvyo-library-core`), `0098-page-kinds` (Add page and the output page), `0101` (operations and assignment), `0102-laser-fixtures` and `0113-fixture-sets-and-pins` (fixtures), `0029-laser-job-preview-and-output` (output group, Toolpath view, Export job file, one builder for file and view), `0150-operation-always-has-numbers`, `0076-single-line-fonts`, `0116-library-sync`.
Related: `0160-embroidery`, `0161-cnc-output` (its virtual GRBL tests the CutcutGo output), `0064-machine-connector`, `0139-laser-cut-path-options` (overcut), `0024` (SVG export).

## Order across the three families

Customer change 2026-11-27: 0159 slice 1, then `0160` slice 1, then `0161` slice 1 (the three MVPs, one "Durchstich"); after that every later slice of the three specs, in the order the lead sets. Frage 427 C still holds: a branch push after each family to look at, one PR at the end.

## User value

As a maker with a Cricut Maker 3 I want to lay out cut and draw work on a page that looks like my cutting mat, grid included, and get a file that Design Space puts on the mat at the size I drew, so that I prepare everything in Curvyo and only press Go in Design Space. As a maker with an open plotter (Silhouette, Graphtec, a generic HPGL cutter) I want the same page to write the plotter's own file so I need no second program.

## What other tools do

Researched 2026-11-27 (lead's plan). Cricut Design Space: mats shown with their grid, but it re-lays out imported parts and owns the machine; the host protocol is encrypted. Silhouette Studio: GPGL, mats, registration marks, closed. Inkscape: HPGL export and the Silhouette extension (GPGL), no mat view, no per-tool settings. LightBurn: no plotters. **What we do better (Proposal):** one drawing feeds laser, plotter, embroidery and CNC pages; the mat is a fixture with the printed grid matched to the maker's real mat; the preview and the file come from the same moves (as `0029`).

## Words

- **Plotter page:** an output page (`PageKind::Output`, ADR 0042) with process id `curvyo.cutter`. Add page offers it next to Laser (`0098`).
- **Mat:** a fixture (`0102`) of a plotter page: the carrier sheet with its size, its cut area, its grip (LightGrip, StandardGrip, StrongGrip, FabricGrip) and its printed grid. Origin top left (Cricut convention).
- **Mat grid:** the printed 1 in grid of the mat drawn on the page; a switch, on by default.
- **Cut:** an operation whose items are cut along their outline by the blade.
- **Draw:** an operation whose items are drawn with a pen along their centre line.
- **Output format:** a field of the machine profile (as `0029` criterion 39 for the laser): `Design Space SVG`, `HPGL`, later `GPGL` and `CutcutGo`.

## Facts needed from the customer (not Fragen)

1. Photos of the customer's 12 x 12 in mats are in (LightGrip, StandardGrip, StrongGrip, 2026-11-27, `docs/design-drafts/2026-11-machine-families/cricut-mat-*.jpg`). Still open: a 12 x 24 in mat photo if he has one; the border offsets are measured from the photos (UX notes).
2. The Design Space version he uses, and one check at the demo that an exported file lands on the mat at its drawn size (criterion 12).
3. The sheet's width and height measured with a ruler, and a photo of a 12 x 24 in mat if he has one (UX M4).
4. For the research slice: USB captures of Design Space sending a small job to the Maker 3 (see slice 7).

## Slices

- **Slice 1: MVP** (criteria 1 to 18, below): plotter page with the Cricut Maker 3 profile and a generic HPGL profile, the 12 x 12 in and 12 x 24 in mats with the grid switch, Cut and Draw operations, path preview, export as Design Space SVG and as HPGL. One `format_version` bump, its number assigned at the final PR (Frage 431 A).
- **Slice 2: tools** (Later; criteria before Ready): Score/Crease (scoring stylus, scoring wheel), Perforate (dashed cut), Deboss, Engrave, Wavy, Foil; per tool pressure, speed, passes, blade offset (drag-knife corner compensation, in HPGL/GPGL only; Design Space does its own), overcut (reuses the overcut of `0139`); fine-point, deep-point, rotary, knife blade with passes; a tool change as a pause (clamp A/B).
- **Slice 3: cutter materials** (Later): a material library per material and tool (pressure, speed, passes) in the family records of ADR 0097, synced as `0116` (Frage 418 A); a test cut (small square with a triangle inside) instead of the laser's test grid; picks as `0146`.
- **Slice 4: more machines and formats** (Later): Silhouette Cameo 4/5 and Portrait over GPGL, Graphtec over GPGL (Frage 417 A; golden files only, the customer has none of them); DXF at mat scale for Brother ScanNCut; Silhouette mats (12 x 12, 12 x 24, Portrait 8 x 12 in, published sizes) **(Proposal)**.
- **Slice 5: CutcutGo** (Later; after `0161` slice 1): G-code output for Cricut machines running the CutcutGo firmware (Frage 416 B). The customer does not reflash his machine: built and tested only against golden files and the virtual GRBL of `0161`, never on hardware.
- **Slice 6: send** (Later): HPGL and GPGL sent over USB or serial through the connector (`0064`, `curvyo-connector-io`), tested against a fake device only.
- **Slice 7: research, Maker 3 direct drive** (Later; Frage 416 B; see below).
- **Slice 8: weeding** (Later): weed lines and weeding boxes for vinyl; kiss-cut versus cut-through as a Cut setting.
- **Later, not scheduled:** print-then-cut registration marks (R-MFG-CUT-002, Could); matless cutting (Smart Materials).

## Acceptance criteria

### Slice 1: MVP

Tolerances: positions and sizes 0.01 mm unless stated; flattening of curves at most 0.01 mm from the exact curve (as `0029`).

**Page, machine, mat**

1. Given Add page, then it offers **Plotter** beside Drawing, Sketch and Laser (`0098`); a new plotter page takes the machine profile **Cricut Maker 3** by default (Proposal) and its page is the **active mat's printed field** (12 x 12 in = 304.8 x 304.8 mm, or 12 x 24 in = 304.8 x 609.6 mm), not editable, with its origin at the mat origin, so the rulers, the panel positions, the export positions (criterion 11) and the mat's cm numbering share one corner; switching the mat resizes the page in the same undo step **(Proposal, UX M1)**. A profile without mats (Generic HPGL) keeps page = work area. Test: Add page, Plotter: a page of kind Output with process `curvyo.cutter`, 304.8 x 304.8 mm; switching to 12 x 24 in makes it 304.8 x 609.6 mm in one step.
2. Given the machine profiles of the plotter family, then two exist built in: **Cricut Maker 3** (output format Design Space SVG, mats 12 x 12 in and 12 x 24 in) and **Generic HPGL cutter** (output format HPGL, work area 600 x 1000 mm (Proposal), origin bottom left, no mat by default). A plotter profile can be copied and edited like a laser profile (the record envelope of ADR 0097; the record key stays `kind = "cutter"`); a laser profile is never offered on a plotter page and the other way round. Test: the profile pick of a plotter page lists exactly the plotter profiles.
3. Given a Maker 3 plotter page, then its Fixtures tab offers the mats **12 x 12 in** and **12 x 24 in**, each with a grip pick (LightGrip, StandardGrip, StrongGrip, FabricGrip; colour of the mat only, no effect on the output); exactly one mat is active per page, 12 x 12 in StandardGrip by default (Proposal). The page shows the mat sheet, its cut area and, between the field edge and the cut area, a muted band. Test: switching to 12 x 24 in changes the drawn mat, the page and the cut area; the grip changes the colour only.
4. Given the active mat, then the page draws it as the customer's mats look (photos `docs/design-drafts/2026-11-machine-families/cricut-mat-lightgrip.jpg`, `cricut-mat-standardgrip.jpg`, `cricut-mats-stack.jpg`; Frage 419 A) as three nested areas: (a) the **sheet** in the grip's colour (LightGrip blue, StandardGrip green, StrongGrip purple, FabricGrip pink) with the notch, a triangular **hole** in the bottom margin, centred (top 9 mm below the field, 7 mm above the sheet edge; **Proposal, UX M2**); (b) the **printed field** of 12 x 12 in (12 x 24 in on the long mat) whose corner next to the Cricut logo and the grip label (top left as fed) is the **mat origin**, the point every position and test is measured from; (c) the machine's **cut area** of 11.5 x 11.5 in (11.5 x 23.5 in on the long mat, published values) centred in the printed field, 0.25 in (6.35 mm) inside it on every side **(Proposal, UX M3)**: the cut area is not printed on the mat, so it is confirmed at the demo by cutting an 11.5 in square placed at 0.25, 0.25 in through Design Space (fact 2). The sheet's margins around the field are in `ux-notes.md` section 5 (from the photos; the sheet's measured size is fact 3). Test: the printed field of the 12 x 12 mat is 304.8 x 304.8 mm, the cut area 292.1 x 292.1 mm from (6.35, 6.35) mm within 0.01 mm.
5. Given the **Mat grid** switch in the mat's fixture row (on by default), then the page draws the mat's printing as on the photos: a white 1 in grid over the printed field (12 x 12 cells of 25.4 mm; 12 x 24 on the long mat) with inch ticks, the grip word, inch numbers along the right and bottom edges counted from the bottom-right corner and turned 180 degrees (to 24 on the long mat's right edge, extrapolated) **(UX M7)**, a cm ruler (1 to 30 on the 12 x 12 mat) along the top and left edges, and a white cross at the field's centre. The printing is drawn in inches and cm whatever the document unit, is never part of the output and never snaps unless snapping to the grid is on (`0060`). Off hides grid, numbers, rulers and cross (the sheet and the cut area stay); the state is stored per page and is one undo step "Mat grid". Test: with the grid on, an export is byte-identical to one with the grid off; on the 12 x 12 mat the cross sits at 152.4, 152.4 mm from the mat origin within 0.01 mm.
6. Given an item outside the active mat's cut area, then the output group shows the check "Outside the cut area" with the item count and Export is blocked, as the laser's work-area check (`0029`). An item partly outside counts as outside. Test: a 10 mm square moved 1 mm past the cut area edge blocks Export.

**Operations**

7. Given a plotter page, then its Operations tab offers two operation kinds, **Cut** and **Draw** (one-line meanings in Frage 428), with assignment as on a laser page (`0101`). A Cut operation follows the outline of each item (a filled closed shape's outline, an open path itself, a text object's glyph outlines, a single-line font text (`0076`) its lines). A Draw operation follows the centre line of each item (a path itself whatever its fill; a single-line font text its strokes; a TrueType text its glyph outlines). Test: a filled circle and a single-line text in one Draw operation give one closed line and the text's strokes; the same items in Cut give the circle outline and the single-line strokes.
8. Given an operation, then it always has numbers (as `0150`): Cut has **Pressure** (1 to 100 %, default from the profile), **Speed** (mm/s, default from the profile) and **Passes** (1 to 10, default 1); Draw has Pressure and Speed. A value outside its range is refused with the range in the field message; each change is one undo step "Edit settings". In Design Space SVG output these numbers are not written (Design Space sets them): on a Design Space page the operation's numbers and the Machine block's Max speed, Default pressure and Default speed are **not shown and not editable**, but kept in storage, so a switch to an HPGL profile finds them **(Proposal, UX M5)**; the row has no unfold and no time. Test: Passes 11 is refused; an HPGL job writes the pressure and speed as in criterion 13.
9. Given several operations, then they run in list order, Draw before Cut by default for a new page (Proposal: a pen first, so a cut part does not move under the pen). Inside an operation the path order is the nearest-next order of `0029`.

**Preview**

10. Given the **Toolpath view** of the page (the toggle of `0029`), then it shows the job's moves from the same builder that writes the file: Cut lines red, Draw lines ink, travel thin dashed, an arrowhead at each path start and a numbered ring per operation, the total path length and, for HPGL, the time from length and speed. Test: the moves drawn equal the moves written (one builder, as `0029` criterion 9).

**Output**

11. Given a Maker 3 page (output format Design Space SVG), when Export job file is pressed, then the app saves one SVG per page named `<project> - <page>.svg`, the laser export's rule **(Proposal, UX M6/C15; the same rule for `.plt`, `.dst` and `.nc`)**, whose `width` and `height` are the mat's size in `in` (for example `width="12in" height="12in"`) with a matching `viewBox`, each item at its position relative to the mat's top-left corner, one group per operation named after it (`id` and `inkscape:label`: "Cut", "Draw", or the operation's name), all paths as strokes without fill, the Draw group in black and each Cut group in one colour. The mat, its grid and other fixtures are not written. Test: golden files `cutter_mat12_cut_draw.svg` and `cutter_mat24_cut.svg` in `tests/fixtures/`.
12. Given that exported file imported into Design Space on the customer's computer, then each part has its drawn size within 0.1 mm (checked at the demo, fact 2). Whether Design Space keeps the mat positions depends on Design Space (it re-lays out parts that are not attached); the file keeps the positions so that Attach in Design Space restores them **(Proposal)**. Draw in Design Space is set by the maker on the Draw group (an SVG cannot carry it). If the size check fails, the fix is in the file's units, not in the drawing.
13. Given a Generic HPGL page, when Export job file is pressed, then the app saves `<project> - <page>.plt`: `IN;` then per operation `VS<speed in cm/s>;` and `FS<force>;` (force: Pressure mapped linearly onto the profile's force range, rounded to a whole number; Generic HPGL cutter 1 to 80 **(Proposal)**) and the paths as `PU x,y;` and `PD x,y,...;` in plotter units of 0.025 mm (40 per mm), integers, origin and axis as the profile says (bottom left, Y up by default), ending with `PU;` and `SP0;` (Proposal). Test: golden files `cutter_hpgl_square.plt` (a 50 mm square: corners at 0,0 and 2000,2000 units) and `cutter_hpgl_cut_draw.plt`.
14. Given Passes n on a Cut operation, then every path of it is written n times in a row, each pass on the same points. Test: Passes 2 doubles the `PD` runs of the golden square.
15. Given a flattened curve, then no written point lies more than 0.01 mm from the exact curve and consecutive equal points (after rounding to plotter units) are dropped.
16. Given an unchanged page, then two exports are byte-identical (no date, no random id).

**Format and libraries**

17. Given a project with a plotter page, then the file stores the page's process, profile copy, mat, grip and grid switch and the plotter operations under a `format_version` assigned at the final PR (Frage 431 A; files saved by branch-push builds may not open after the merge); a build of the format before refuses it with the newer-version message; a file of the format before opens unchanged. Golden: `plotter_page_v<N>.curvyo` and a damaged variant `plotter_page_damaged_v<N>.curvyo` (unknown mat size, pressure out of range: read as default with a notice).
18. Given plotter profiles in the library, then they sync across the maker's devices like laser profiles (`0116`), with the per-item sync toggle.

### Slice 7: research, Maker 3 direct drive (exit criteria)

R1. Inputs: USB captures from the customer's Maker 3 while Design Space sends (a) one 20 mm square Cut and (b) the same square Draw, recorded with `usbmon` and Wireshark (fact 4). Without captures the slice does not start.
R2. Time-box: 3 working days of analysis **(Proposal)**. The result is a written finding in the spec folder (`research-maker3.md`): what the captures show, what is encrypted, what would be needed.
R3. Success means: Curvyo can make the Maker 3 cut a 20 mm square with nothing from Cricut shipped (no key, firmware or Design Space binary in the repo or the app, see Frage 429). Then a follow-up slice with its own criteria is drafted (send through the connector).
R4. Failure (encrypted beyond reach, or a Cricut key needed and Frage 429 stays A): nothing ships except the finding; Design Space SVG stays the route for the Maker 3, and the spec records the outcome.

## What changes accepted behaviour

- Add page gains the Plotter kind (`0098`); the profile record gains `kind = "cutter"` (ADR 0097); laser pages and files do not change.
- The output group, the Toolpath view and Export job file of `0029` serve plotter pages too; the laser's checks and numbers are untouched.

## Out of scope

- Driving the Maker 3 directly in slice 1 (slice 7 is research with an exit).
- Any Cricut key, firmware or Design Space binary in the repo or the app; reflashing the customer's machine.
- Registration marks, print-then-cut, matless cutting, weeding (slice 8), nesting (R-MFG-004).
- Bluetooth.

## Questions for the customer

Frage 428 (OP-MEANING-MVP): Meaning of the new MVP operations (all three families)
Answered 2026-11-27: A (customer).
- A: as written: **Cut** = the blade cuts along the outline; **Draw** = the pen follows the centre line; **Running stitch** = single straight stitches along the line; **Tatami fill** = the area filled with parallel, offset rows of stitches; **Satin** = a zigzag column along the line, as wide as its stroke; **Profile outside/inside** = the end mill runs around the outline, offset outward or inward by the tool radius.
- B: changes per operation (name them).
Recommendation A. Default if unanswered: A.

Frage 429 (CRICUT-KEY): What if the Maker 3 can only be driven with a key taken from Cricut's software?
Answered 2026-11-27: A, for now (customer).
- A: stop: write down the finding; Design Space SVG stays the route.
- B: Curvyo ships nothing of Cricut's but reads a key the maker enters from his own installation.
- C: keep researching without a time-box.
Recommendation A (legally clean, no dependency on Cricut's updates). Default if unanswered: A.

## Customer answers 2026-11-27 (machine families, all three specs)

- Frage 430 (LICENCE-CONSEQUENCE) A: Curvyo stays AGPL-3.0-or-later for good, commercial use included; the Ink/Stitch port as a GPL-3 crate is accepted.
- Frage 431 (MF-BRANCH-FILES) A: files saved by builds of the branch pushes may not open after the merge; the `format_version` numbers are assigned at the final PR.
- Frage 432 (MF-EMB-READ) A: an imported embroidery file becomes paths with a Manual stitch; viewing it is what matters, editing is secondary.
- Frage 433 (MF-TOOLS-DIR) A: CNC tools get their own `tools/` library, synced as `0116`.
- MF-REC-KIND: default A (technical, architect): the record key stays `kind` (`kind = "cutter"`, `"embroidery"`, `"cnc"`), no `family` key ([ADR 0097](../../docs/adr/0097-cutting-plotter-embroidery-and-cnc-families.md)).

## UX notes

See [`ux-notes.md`](ux-notes.md) (mismatches M1 to M8 applied above as Proposals).

## Links

Requirements: R-MFG-001, R-MFG-002, R-MFG-003, R-MFG-CUT-001, R-MFG-CUT-003 to R-MFG-CUT-010, R-MAT-001, R-MAT-005. Plan: `docs/design-drafts/2026-11-machine-families/plan.md`. ADR: [ADR 0097](../../docs/adr/0097-cutting-plotter-embroidery-and-cnc-families.md). PR: -
