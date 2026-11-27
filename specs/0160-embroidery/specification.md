# Embroidery: an embroidery output page with hoops, running stitch, tatami fill and satin, a stitch simulation and DST for the W6 EU7

Status: Ready (slice 1 MVP; later slices stay Draft until their criteria are written) (2026-11-27; `adrs.md` and `ux-notes.md` in, UX mismatches applied as Proposals; earlier: Draft (2026-11-27; customer answers to Fragen 420 to 424 and 427 in; Frage 428 answered A (2026-11-27), see `0159`; waits for `adrs.md` (ADR 0097 written) and UX notes))
Priority: Should
Origin: Customer request 2026-11-27 (translated): three new machine families, embroidery second; the customer's machine is a **W6 EU7** (W6 N 5000 Exklusive / N 8000 Exklusive family) that reads DST from a USB stick. Customer change of 2026-11-27: **first a thin end-to-end slice through all three families, then everything else.** Plan and answers: `docs/design-drafts/2026-11-machine-families/plan.md`. Everything marked **(Proposal)** is the product owner's and vetoable at the demo.
Depends on: [ADR 0097](../../docs/adr/0097-cutting-plotter-embroidery-and-cnc-families.md), `0098-page-kinds`, `0101` (operations and assignment), `0102-laser-fixtures` and `0113-fixture-sets-and-pins` (hoops as fixtures), `0029-laser-job-preview-and-output` (output group, Toolpath view and its play slider, Export job file, one builder for file and view), `0150-operation-always-has-numbers`, `0110-pen-modes-variable-stroke` (width profile, satin later), `0117-centre-lines` (centre lines for satin later), `0076-single-line-fonts` and the font library (lettering), `0116-library-sync`.
Related: `0159-cutting-plotter` (slice 1 comes first), `0161-cnc-output`, `0022-color-management` (thread palettes), `0033-stroke-brushes`.

## Order across the three families

`0159` slice 1, then this spec's slice 1, then `0161` slice 1; after that every later slice of the three, in the order the lead sets (customer change 2026-11-27; Frage 427 C: a branch push after each family, one PR at the end).

## User value

As a maker with a W6 EU7 I want to turn a drawing on a page that shows my hoop into running stitches, fills and satin columns, watch the machine sew it in a simulation, and save a DST for the USB stick, so that I embroider my own designs without buying a separate embroidery program.

## What other tools do

Ink/Stitch (GPL-3, Inkscape extension): the reference for stitch quality; parameters per object, simulator, many formats through pyembroidery (MIT). Hatch, Embrilliance, Wilcom: closed, expensive, auto-digitising. W6's own software: none worth naming for Linux. **What we do better (Proposal):** stitches are generated per operation on an output page, like laser and plotter, from the same drawing; the simulation and the file come from one stitch list; Ink/Stitch's algorithms are ported (Frage 421 C) so quality starts at its level.

## Words

- **Embroidery page:** an output page with process id `curvyo.embroidery`.
- **Hoop:** a fixture of the page: the frame and its sewing field (the area the needle may reach), centred on the page.
- **Stitch list:** what the machine does, in order: stitch (needle down at a point), jump (move without stitching), trim, colour change (machine stops, the maker changes thread), end. Lock stitches (tie-in, tie-off) are short stitch runs at the start and end of a block.
- **Running stitch:** single stitches of a set length along a line.
- **Tatami fill:** a closed area filled with parallel rows of stitches at an angle, the stitch ends of neighbouring rows offset so no visible line forms.
- **Satin:** a zigzag column along a line, as wide as the line's stroke.
- **Thread colour:** a colour on each operation; one operation is one colour block.

## Facts needed from the customer (not Fragen)

1. The hoops of his EU7: 150 x 250 mm and 100 x 100 mm are the default until he confirms the sewing fields (Frage 420).
2. The machine's top speed in stitches per minute, for the simulation time (default 600 per minute **(Proposal)**).
3. One sewn test at the demo: the slice 1 golden design on a scrap of cotton with stabiliser.

## Slices

- **Slice 1: MVP** (criteria 1 to 20, below): embroidery page with the EU7 hoops, Running stitch, Tatami fill and simple Satin operations (ported from Ink/Stitch), lock stitches, trims and colour changes, stitch simulation, DST export. One `format_version` bump, its number assigned at the final PR (Frage 431 A).
- **Slice 2: line stitches** (Later; criteria before Ready): bean, manual, zigzag, E-stitch, ripple.
- **Slice 3: satin columns** (Later): two rails, or one path with the variable width of `0110` (from centre lines of `0117`); underlays (centre walk, contour, zigzag); pull compensation; short stitches.
- **Slice 4: fills** (Later): offset/contour fill ("Füllung mit Offset"), guided, meander, circular, linear gradient; fill underlay; end rows; tatami staggers.
- **Slice 5: stitch order and direction** (Later): travel, jumps versus trims, start and end points, show and reverse path direction (R-MFG-EMB-005); stitch direction right to left (Frage 423 A, Proposal reading).
- **Slice 6: patterns** (Later): fill patterns and pattern stitches.
- **Slice 7: lettering** (Later; Fragen 422 C, 423): text from the normal font library (TrueType as fill or satin, single-line fonts as running stitch), Ink/Stitch fonts added to that library where their licence allows; letters routed for few trims; no extra font controls in the embroidery view.
- **Slice 8: formats** (Later; Frage 424 A): write and read JEF, EXP, PES/PEC, VP3, XXX, U01 (DST is in slice 1); golden files each.
- **Slice 9: fabrics** (Later; Frage 418 A): fabric presets (density, underlay, pull compensation, stabiliser) in the family records of ADR 0097, synced as `0116`.
- **Slice 10: read embroidery files** (Later; Frage 432 A): an imported DST (and the formats of slice 8) becomes paths with a **Manual stitch** on an embroidery page, one per colour block, so the design can be viewed, placed and re-exported; editing is secondary.

## Acceptance criteria

### Slice 1: MVP

Tolerances: stitch positions 0.1 mm (the DST unit); generated geometry 0.01 mm before rounding.

**Page, machine, hoop**

1. Given Add page, then it offers **Embroidery**; a new embroidery page takes the profile **W6 EU7** (Proposal) and its page size is the profile's largest sewing field plus 20 mm on every side (Proposal), not editable; the drawn hoop frame fills that margin (`ux-notes.md`, UX M5). Test: a page of kind Output with process `curvyo.embroidery`.
2. Given the built-in profile W6 EU7, then it has output format DST, hoops **150 x 250 mm** and **100 x 100 mm** (sewing fields; fact 1), maximum stitch length 12.1 mm (the DST limit) and a speed of 600 stitches per minute (fact 2). It can be copied and edited like a laser profile (record `kind = "embroidery"`, ADR 0097) and syncs as `0116`.
3. Given an embroidery page, then its Fixtures tab offers the profile's hoops; exactly one is active, 150 x 250 mm by default; the page draws the hoop frame and its sewing field, centred. Test: switching hoops changes the drawn field; the hoop is never written to the file.
4. Given a stitch outside the active hoop's sewing field, then the output group shows "Outside the hoop" with the operation and Export is blocked (as the laser's work-area check). Test: a fill 1 mm past the 100 x 100 field blocks Export.

**Operations**

5. Given an embroidery page, then its Operations tab offers three kinds: **Running stitch**, **Tatami fill** and **Satin** (meanings: Frage 428), with assignment as on a laser page (`0101`), and a thread colour on each operation (default: the stroke or fill colour of its first item, Proposal). Running stitch and Satin take an item's line (an open path, a closed path's outline, a single-line font text's strokes); Tatami fill takes closed areas (a filled closed shape, a compound path with holes, a TrueType text's glyphs).
6. Given an operation, then it always has numbers (as `0150`), defaults from Ink/Stitch: Running stitch: **Stitch length** 2.5 mm (1 to 10), **Repeats** 1 (1, 3, 5: a triple stitch runs forward, back, forward). Tatami fill: **Angle** 0 degrees (0 to 179), **Row spacing** 0.25 mm (0.1 to 2), **Max stitch length** 3 mm (1 to 10). Satin: **Zigzag spacing** 0.4 mm (0.2 to 2); the width is the item's stroke width (constant in slice 1). Out-of-range values are refused with the range; each change is one undo step "Edit settings".
7. Given a Running stitch on a 100 mm straight line with length 2.5 mm, then it gives 41 needle points from end to end, 2.5 mm apart within 0.1 mm; on a curve the points lie on the curve within 0.1 mm and no stitch is longer than the set length.
8. Given a Tatami fill on a 50 x 30 mm rectangle, angle 0, spacing 0.25 mm, then the rows run horizontally, 0.25 mm apart within 0.05 mm, stay inside the outline (no needle point more than 0.1 mm outside), no stitch is longer than the max length, and the stitch ends of rows next to each other are offset (Ink/Stitch's staggers). A shape with a hole leaves the hole unstitched; rows that must cross the hole travel by running stitches inside the area or jump (as the port does).
9. Given a Satin on a 40 mm line with a 3 mm stroke, then the needle alternates between the two sides 1.5 mm left and right of the line, each zigzag step advances 0.4 mm along the line, and the column covers the line from end to end. A stroke narrower than 0.5 mm runs as a running stitch, and the output group shows a warn check "Satin 1: 1 item is too narrow and runs as running stitch." (operation and count) while it is true; it does not block Export **(Proposal, UX M2)**.
10. Given the generated stitches, then each block (one item of one operation) starts with a tie-in and ends with a tie-off of three short stitches (Proposal, as Ink/Stitch's default); between two blocks of the same colour a travel longer than 5 mm (Proposal) is a trim, a shorter one a jump; between operations of different colours a colour change; the stitch list ends with an end. Operations run in list order, items inside one by the nearest-next order.
11. Given the port (Frage 421 C), then the algorithms of criteria 7 to 10 live in their own crate under the GPL-3 licence Ink/Stitch's files carry (exact SPDX id: architect) with an attribution header per file naming the Ink/Stitch source file; the rest of Curvyo stays AGPL-3.0-or-later; `cargo deny` allows exactly that crate's licence. (Crate and boundary: architect, ADR 0097 or its own ADR.)

**Simulation**

12. Given the Toolpath view of the page (`0029`), then it shows the stitch list from the same builder that writes the file: stitches in their thread colour, jumps dashed, trims and colour changes marked, and a summary: stitch count, colour count, trims, thread length in m (stitch lengths summed) and time at the profile's speed. Test: the summary of the golden design equals the counts read back from its DST.
13. Given the play slider of the Toolpath view, then Play sews the stitches in order at a chosen speed: 1x is the profile's machine speed (600 stitches per minute), then 4x and 16x; default 16x **(Proposal, UX M1)**, Pause stops, dragging the slider shows the design up to that stitch, and the readout shows the stitch number and its colour. The state is session state, not saved, not an undo step.

**Output**

14. Given an embroidery page with output format DST, when Export job file is pressed, then the app saves `<project> - <page>.dst` (the laser export's rule, as `0159` criterion 11): a 512-byte header (`LA:` the page name cut to 16 characters, `ST:` stitch count, `CO:` colour changes, `+X`, `-X`, `+Y`, `-Y` extents in 0.1 mm, `AX`, `AY`, `MX`, `MY`, `PD`, padded with spaces as the format requires), then the stitch records in 0.1 mm relative steps, ending with the end record. Test: golden files `embroidery_running.dst`, `embroidery_tatami.dst`, `embroidery_satin.dst` and `embroidery_mvp_two_colours.dst` in `tests/fixtures/`. Positions are written relative to the **hoop centre** (the page centre), where the W6 places the design's start; the page and its rulers count from the top-left, so the export translates by the hoop centre (**UX M4**; transform: `adrs.md`). Test: a 10 mm square centred in the hoop has extents +X = -X = +Y = -Y = 50 (0.1 mm units).
15. Given a step longer than 12.1 mm in X or Y, then it is split into jumps of at most 12.1 mm; a stitch is never longer than 12.1 mm. Test: a 30 mm travel is written as three jumps.
16. Given a trim, then it is written as the DST convention of a jump sequence the W6 reads as a trim (exact sequence fixed in the golden file, checked at the demo); a colour change as the stop record. DST has no thread colours: the colours stay in the project; the export writes nothing else.
17. Given the exported DST read back by Curvyo's own DST reader (test only in slice 1), then every needle point equals the stitch list's point within 0.1 mm and the header extents equal the points' bounding box. Test: read-back of the four golden files.
18. Given an unchanged page, then two exports are byte-identical.
19. Given the golden design `embroidery_mvp_two_colours` (a running stitch outline, a tatami fill inside, a satin line, two colours) on a USB stick in the customer's EU7, then the machine shows it, sews it with one colour stop, and the result matches the simulation (customer check, fact 3).

**Format**

20. Given a project with an embroidery page, then the file stores the page's process, profile copy, active hoop and the operations with their numbers and colours under a `format_version` assigned at the final PR (Frage 431 A; files saved by branch-push builds may not open after the merge); the build before refuses it with the newer-version message; a file of the format before opens unchanged. Golden: `embroidery_page_v<N>.curvyo` and `embroidery_page_damaged_v<N>.curvyo` (unknown hoop, row spacing out of range: read as default with a notice).

## What changes accepted behaviour

- Add page gains the Embroidery kind; the profile record gains `kind = "embroidery"` (ADR 0097).
- A GPL-3 crate (the Ink/Stitch port) enters the workspace (Frage 421 C; Frage 430 A: Curvyo stays AGPL-3.0-or-later for good); `deny.toml` changes for it.

## Out of scope

- Auto-digitising a bitmap; appliqué; 3D puff; multi-hoop splitting.
- A separate font UI in the embroidery view (Fragen 422 C, 423).
- Reading any format in the app in slice 1 (the DST reader is a test tool until slice 10, Frage 432 A).
- Driving the machine directly (it reads files from a USB stick).

## Customer answers 2026-11-27 (machine families, all three specs)

- Frage 430 (LICENCE-CONSEQUENCE) A: Curvyo stays AGPL-3.0-or-later for good, commercial use included; the Ink/Stitch port as a GPL-3 crate is accepted.
- Frage 431 (MF-BRANCH-FILES) A: files saved by builds of the branch pushes may not open after the merge; the `format_version` numbers are assigned at the final PR.
- Frage 432 (MF-EMB-READ) A: an imported embroidery file becomes paths with a Manual stitch; viewing it is what matters, editing is secondary.
- Frage 433 (MF-TOOLS-DIR) A: CNC tools get their own `tools/` library, synced as `0116`.
- MF-REC-KIND: default A (technical, architect): the record key stays `kind` (`kind = "cutter"`, `"embroidery"`, `"cnc"`), no `family` key ([ADR 0097](../../docs/adr/0097-cutting-plotter-embroidery-and-cnc-families.md)).

## UX notes

See [`ux-notes.md`](ux-notes.md) (mismatches M1 to M6 applied above as Proposals).

## Links

Requirements: R-MFG-001 to R-MFG-003, R-MFG-009, R-MFG-EMB-001 to R-MFG-EMB-003, R-MFG-EMB-006 to R-MFG-EMB-012 (R-MFG-EMB-004, -005 in later slices), R-MAT-001, R-MAT-005. Plan: `docs/design-drafts/2026-11-machine-families/plan.md`. ADR: [ADR 0097](../../docs/adr/0097-cutting-plotter-embroidery-and-cnc-families.md). PR: -
